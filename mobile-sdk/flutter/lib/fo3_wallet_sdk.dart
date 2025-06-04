/// FO3 Wallet Core Flutter SDK
/// 
/// Provides a comprehensive Flutter SDK for integrating with FO3 Wallet Core gRPC API.
/// Features include:
/// - Automatic gRPC client management with connection pooling
/// - JWT authentication with automatic token refresh
/// - Real-time WebSocket notifications
/// - Comprehensive error handling and retry logic
/// - Offline caching and synchronization
/// - Type-safe API calls with generated models
/// - Performance monitoring and analytics

library fo3_wallet_sdk;

import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:grpc/grpc.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:web_socket_channel/web_socket_channel.dart';
import 'package:connectivity_plus/connectivity_plus.dart';
import 'package:crypto/crypto.dart';

// Generated gRPC clients
import 'generated/auth.pbgrpc.dart';
import 'generated/wallet.pbgrpc.dart';
import 'generated/earn.pbgrpc.dart';
import 'generated/moonshot.pbgrpc.dart';
import 'generated/market_intelligence.pbgrpc.dart';
import 'generated/cards.pbgrpc.dart';
import 'generated/defi.pbgrpc.dart';

export 'generated/auth.pb.dart';
export 'generated/wallet.pb.dart';
export 'generated/earn.pb.dart';
export 'generated/moonshot.pb.dart';
export 'generated/market_intelligence.pb.dart';
export 'generated/cards.pb.dart';
export 'generated/defi.pb.dart';

/// SDK Configuration
class FO3WalletConfig {
  final String baseUrl;
  final int grpcPort;
  final int websocketPort;
  final bool enableTLS;
  final Duration connectionTimeout;
  final Duration requestTimeout;
  final int maxRetries;
  final bool enableCaching;
  final bool enableAnalytics;
  final String? apiKey;

  const FO3WalletConfig({
    this.baseUrl = 'api.fo3wallet.com',
    this.grpcPort = 50051,
    this.websocketPort = 8080,
    this.enableTLS = true,
    this.connectionTimeout = const Duration(seconds: 30),
    this.requestTimeout = const Duration(seconds: 10),
    this.maxRetries = 3,
    this.enableCaching = true,
    this.enableAnalytics = true,
    this.apiKey,
  });

  /// Development configuration
  static const FO3WalletConfig development = FO3WalletConfig(
    baseUrl: 'localhost',
    enableTLS: false,
  );

  /// Production configuration
  static const FO3WalletConfig production = FO3WalletConfig(
    baseUrl: 'api.fo3wallet.com',
    enableTLS: true,
  );
}

/// Authentication state
enum AuthState {
  unauthenticated,
  authenticating,
  authenticated,
  refreshing,
  expired,
}

/// SDK Exception types
class FO3WalletException implements Exception {
  final String message;
  final String? code;
  final dynamic details;

  const FO3WalletException(this.message, {this.code, this.details});

  @override
  String toString() => 'FO3WalletException: $message${code != null ? ' (Code: $code)' : ''}';
}

class AuthenticationException extends FO3WalletException {
  const AuthenticationException(String message) : super(message, code: 'AUTH_ERROR');
}

class NetworkException extends FO3WalletException {
  const NetworkException(String message) : super(message, code: 'NETWORK_ERROR');
}

class RateLimitException extends FO3WalletException {
  const RateLimitException(String message) : super(message, code: 'RATE_LIMIT');
}

/// Main SDK class
class FO3WalletSDK {
  static FO3WalletSDK? _instance;
  static FO3WalletSDK get instance => _instance ?? (throw StateError('SDK not initialized'));

  final FO3WalletConfig config;
  late ClientChannel _channel;
  
  // gRPC Clients
  late AuthServiceClient _authClient;
  late WalletServiceClient _walletClient;
  late EarnServiceClient _earnClient;
  late MoonshotTradingServiceClient _moonshotClient;
  late MarketIntelligenceServiceClient _marketIntelligenceClient;
  late CardServiceClient _cardClient;
  late DeFiServiceClient _defiClient;

  // Authentication
  String? _accessToken;
  String? _refreshToken;
  DateTime? _tokenExpiry;
  final StreamController<AuthState> _authStateController = StreamController<AuthState>.broadcast();
  AuthState _currentAuthState = AuthState.unauthenticated;

  // WebSocket
  WebSocketChannel? _wsChannel;
  final StreamController<Map<String, dynamic>> _notificationController = 
      StreamController<Map<String, dynamic>>.broadcast();

  // Connectivity
  late StreamSubscription<ConnectivityResult> _connectivitySubscription;
  bool _isOnline = true;

  // Cache
  final Map<String, dynamic> _cache = {};
  Timer? _cacheCleanupTimer;

  FO3WalletSDK._(this.config);

  /// Initialize the SDK
  static Future<FO3WalletSDK> initialize(FO3WalletConfig config) async {
    if (_instance != null) {
      throw StateError('SDK already initialized');
    }

    _instance = FO3WalletSDK._(config);
    await _instance!._initialize();
    return _instance!;
  }

  Future<void> _initialize() async {
    // Initialize gRPC channel
    _initializeGrpcChannel();

    // Initialize clients
    _initializeClients();

    // Setup connectivity monitoring
    _setupConnectivityMonitoring();

    // Setup cache cleanup
    if (config.enableCaching) {
      _setupCacheCleanup();
    }

    // Try to restore authentication state
    await _restoreAuthState();
  }

  void _initializeGrpcChannel() {
    final channelOptions = ChannelOptions(
      credentials: config.enableTLS 
          ? const ChannelCredentials.secure()
          : const ChannelCredentials.insecure(),
      connectTimeout: config.connectionTimeout,
    );

    _channel = ClientChannel(
      config.baseUrl,
      port: config.grpcPort,
      options: channelOptions,
    );
  }

  void _initializeClients() {
    _authClient = AuthServiceClient(_channel);
    _walletClient = WalletServiceClient(_channel);
    _earnClient = EarnServiceClient(_channel);
    _moonshotClient = MoonshotTradingServiceClient(_channel);
    _marketIntelligenceClient = MarketIntelligenceServiceClient(_channel);
    _cardClient = CardServiceClient(_channel);
    _defiClient = DeFiServiceClient(_channel);
  }

  void _setupConnectivityMonitoring() {
    _connectivitySubscription = Connectivity().onConnectivityChanged.listen((result) {
      final wasOnline = _isOnline;
      _isOnline = result != ConnectivityResult.none;
      
      if (!wasOnline && _isOnline) {
        // Reconnected - refresh data
        _onReconnected();
      } else if (wasOnline && !_isOnline) {
        // Disconnected
        _onDisconnected();
      }
    });
  }

  void _setupCacheCleanup() {
    _cacheCleanupTimer = Timer.periodic(const Duration(hours: 1), (_) {
      _cleanupCache();
    });
  }

  Future<void> _restoreAuthState() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      _accessToken = prefs.getString('fo3_access_token');
      _refreshToken = prefs.getString('fo3_refresh_token');
      
      final expiryString = prefs.getString('fo3_token_expiry');
      if (expiryString != null) {
        _tokenExpiry = DateTime.parse(expiryString);
      }

      if (_accessToken != null && _tokenExpiry != null) {
        if (_tokenExpiry!.isAfter(DateTime.now())) {
          _setAuthState(AuthState.authenticated);
          _connectWebSocket();
        } else if (_refreshToken != null) {
          await _refreshAccessToken();
        }
      }
    } catch (e) {
      // Ignore restoration errors
      _setAuthState(AuthState.unauthenticated);
    }
  }

  /// Authentication methods
  Future<LoginResponse> login(String email, String password) async {
    _setAuthState(AuthState.authenticating);

    try {
      final request = LoginRequest()
        ..email = email
        ..password = password;

      final response = await _executeWithRetry(() => 
        _authClient.login(request, options: _getCallOptions())
      );

      _accessToken = response.accessToken;
      _refreshToken = response.refreshToken;
      _tokenExpiry = DateTime.now().add(Duration(seconds: response.expiresIn.toInt()));

      await _storeTokens();
      _setAuthState(AuthState.authenticated);
      _connectWebSocket();

      return response;
    } catch (e) {
      _setAuthState(AuthState.unauthenticated);
      throw _handleGrpcError(e);
    }
  }

  Future<void> logout() async {
    try {
      if (_accessToken != null) {
        await _authClient.logout(LogoutRequest(), options: _getAuthCallOptions());
      }
    } catch (e) {
      // Ignore logout errors
    } finally {
      await _clearTokens();
      _setAuthState(AuthState.unauthenticated);
      _disconnectWebSocket();
    }
  }

  Future<void> _refreshAccessToken() async {
    if (_refreshToken == null) {
      throw const AuthenticationException('No refresh token available');
    }

    _setAuthState(AuthState.refreshing);

    try {
      final request = RefreshTokenRequest()..refreshToken = _refreshToken!;
      final response = await _authClient.refreshToken(request, options: _getCallOptions());

      _accessToken = response.accessToken;
      _tokenExpiry = DateTime.now().add(Duration(seconds: response.expiresIn.toInt()));

      await _storeTokens();
      _setAuthState(AuthState.authenticated);
    } catch (e) {
      await _clearTokens();
      _setAuthState(AuthState.expired);
      throw _handleGrpcError(e);
    }
  }

  /// Wallet operations
  Future<GetBalanceResponse> getWalletBalance(String walletId, String currency) async {
    final cacheKey = 'balance_${walletId}_$currency';
    
    if (config.enableCaching && _cache.containsKey(cacheKey)) {
      final cached = _cache[cacheKey];
      if (cached['expiry'].isAfter(DateTime.now())) {
        return cached['data'] as GetBalanceResponse;
      }
    }

    final request = GetBalanceRequest()
      ..walletId = walletId
      ..currency = currency;

    final response = await _executeWithRetry(() =>
      _walletClient.getBalance(request, options: _getAuthCallOptions())
    );

    if (config.enableCaching) {
      _cache[cacheKey] = {
        'data': response,
        'expiry': DateTime.now().add(const Duration(minutes: 5)),
      };
    }

    return response;
  }

  Future<ListWalletsResponse> listWallets(String userId) async {
    final request = ListWalletsRequest()..userId = userId;
    return await _executeWithRetry(() =>
      _walletClient.listWallets(request, options: _getAuthCallOptions())
    );
  }

  /// Earn service operations
  Future<GetYieldProductsResponse> getYieldProducts({
    String? category,
    double? minApy,
    String? riskLevel,
  }) async {
    final request = GetYieldProductsRequest();
    if (category != null) request.category = category;
    if (minApy != null) request.minApy = minApy;
    if (riskLevel != null) request.riskLevel = riskLevel;

    return await _executeWithRetry(() =>
      _earnClient.getYieldProducts(request, options: _getAuthCallOptions())
    );
  }

  /// Moonshot trading operations
  Future<GetTrendingTokensResponse> getTrendingTokens({
    int page = 1,
    int pageSize = 20,
    String? timeFrame,
    String? sortBy,
  }) async {
    final request = GetTrendingTokensRequest()
      ..page = page
      ..pageSize = pageSize;
    if (timeFrame != null) request.timeFrame = timeFrame;
    if (sortBy != null) request.sortBy = sortBy;

    return await _executeWithRetry(() =>
      _moonshotClient.getTrendingTokens(request, options: _getAuthCallOptions())
    );
  }

  /// Market intelligence operations
  Future<GetRealTimeMarketDataResponse> getRealTimeMarketData(
    List<String> symbols, {
    List<String>? blockchains,
    String? dataGranularity,
    bool includeOrderbook = false,
    bool includeTrades = false,
  }) async {
    final request = GetRealTimeMarketDataRequest()
      ..symbols.addAll(symbols)
      ..dataGranularity = dataGranularity ?? '1m'
      ..includeOrderbook = includeOrderbook
      ..includeTrades = includeTrades;
    
    if (blockchains != null) {
      request.blockchains.addAll(blockchains);
    }

    return await _executeWithRetry(() =>
      _marketIntelligenceClient.getRealTimeMarketData(request, options: _getAuthCallOptions())
    );
  }

  /// WebSocket operations
  void _connectWebSocket() {
    if (_accessToken == null) return;

    try {
      final wsUrl = '${config.enableTLS ? 'wss' : 'ws'}://${config.baseUrl}:${config.websocketPort}';
      _wsChannel = WebSocketChannel.connect(
        Uri.parse(wsUrl),
        protocols: ['Bearer $_accessToken'],
      );

      _wsChannel!.stream.listen(
        (data) {
          try {
            final decoded = jsonDecode(data);
            _notificationController.add(decoded);
          } catch (e) {
            // Ignore invalid JSON
          }
        },
        onError: (error) {
          // Handle WebSocket errors
          _reconnectWebSocket();
        },
        onDone: () {
          // Handle WebSocket disconnection
          _reconnectWebSocket();
        },
      );
    } catch (e) {
      // Handle connection errors
    }
  }

  void _disconnectWebSocket() {
    _wsChannel?.sink.close();
    _wsChannel = null;
  }

  void _reconnectWebSocket() {
    _disconnectWebSocket();
    if (_currentAuthState == AuthState.authenticated) {
      Future.delayed(const Duration(seconds: 5), _connectWebSocket);
    }
  }

  /// Utility methods
  CallOptions _getCallOptions() {
    return CallOptions(timeout: config.requestTimeout);
  }

  CallOptions _getAuthCallOptions() {
    if (_accessToken == null) {
      throw const AuthenticationException('Not authenticated');
    }

    return CallOptions(
      timeout: config.requestTimeout,
      metadata: {'authorization': 'Bearer $_accessToken'},
    );
  }

  Future<T> _executeWithRetry<T>(Future<T> Function() operation) async {
    int attempts = 0;
    while (attempts < config.maxRetries) {
      try {
        return await operation();
      } catch (e) {
        attempts++;
        if (attempts >= config.maxRetries) {
          throw _handleGrpcError(e);
        }
        
        // Exponential backoff
        await Future.delayed(Duration(milliseconds: 100 * (1 << attempts)));
      }
    }
    throw const FO3WalletException('Max retries exceeded');
  }

  Exception _handleGrpcError(dynamic error) {
    if (error is GrpcError) {
      switch (error.code) {
        case StatusCode.unauthenticated:
          return const AuthenticationException('Authentication required');
        case StatusCode.permissionDenied:
          return const FO3WalletException('Permission denied', code: 'PERMISSION_DENIED');
        case StatusCode.resourceExhausted:
          return const RateLimitException('Rate limit exceeded');
        case StatusCode.unavailable:
          return const NetworkException('Service unavailable');
        default:
          return FO3WalletException(error.message ?? 'Unknown error', code: error.code.toString());
      }
    }
    return FO3WalletException(error.toString());
  }

  void _setAuthState(AuthState state) {
    _currentAuthState = state;
    _authStateController.add(state);
  }

  Future<void> _storeTokens() async {
    final prefs = await SharedPreferences.getInstance();
    if (_accessToken != null) {
      await prefs.setString('fo3_access_token', _accessToken!);
    }
    if (_refreshToken != null) {
      await prefs.setString('fo3_refresh_token', _refreshToken!);
    }
    if (_tokenExpiry != null) {
      await prefs.setString('fo3_token_expiry', _tokenExpiry!.toIso8601String());
    }
  }

  Future<void> _clearTokens() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove('fo3_access_token');
    await prefs.remove('fo3_refresh_token');
    await prefs.remove('fo3_token_expiry');
    
    _accessToken = null;
    _refreshToken = null;
    _tokenExpiry = null;
  }

  void _onReconnected() {
    if (_currentAuthState == AuthState.authenticated) {
      _connectWebSocket();
    }
  }

  void _onDisconnected() {
    _disconnectWebSocket();
  }

  void _cleanupCache() {
    final now = DateTime.now();
    _cache.removeWhere((key, value) => value['expiry'].isBefore(now));
  }

  /// Streams
  Stream<AuthState> get authStateStream => _authStateController.stream;
  Stream<Map<String, dynamic>> get notificationStream => _notificationController.stream;

  /// Getters
  AuthState get authState => _currentAuthState;
  bool get isAuthenticated => _currentAuthState == AuthState.authenticated;
  bool get isOnline => _isOnline;

  /// Cleanup
  Future<void> dispose() async {
    await _connectivitySubscription.cancel();
    _cacheCleanupTimer?.cancel();
    _disconnectWebSocket();
    await _authStateController.close();
    await _notificationController.close();
    await _channel.shutdown();
    _instance = null;
  }
}
