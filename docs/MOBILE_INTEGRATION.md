# FO3 Wallet Core Mobile Integration Guide

**Target Platforms:** Flutter (Android/iOS) and Swift (iOS)  
**Protocol:** gRPC with TLS  
**Authentication:** JWT + RBAC  
**Real-time:** WebSocket connections  

## 🎯 Overview

This guide provides comprehensive instructions for integrating FO3 Wallet Core gRPC API into mobile applications, with specific examples for Flutter and Swift iOS development.

## 📋 Prerequisites

### Flutter Development
- Flutter SDK 3.0+
- Dart 2.17+
- gRPC Dart package
- Protocol Buffers compiler

### Swift iOS Development
- Xcode 14+
- iOS 13.0+
- Swift 5.7+
- gRPC Swift package
- SwiftProtobuf

## 🚀 Quick Start

### Flutter Setup

#### 1. Add Dependencies

```yaml
# pubspec.yaml
dependencies:
  grpc: ^3.2.4
  protobuf: ^3.1.0
  fixnum: ^1.1.0
  web_socket_channel: ^2.4.0
  shared_preferences: ^2.2.0
  dio: ^5.3.0

dev_dependencies:
  protoc_plugin: ^21.1.2
```

#### 2. Generate gRPC Client Code

```bash
# Install protoc compiler
brew install protobuf  # macOS
# or
sudo apt-get install protobuf-compiler  # Ubuntu

# Generate Dart code from proto files
mkdir -p lib/generated
protoc --dart_out=grpc:lib/generated \
  --proto_path=../proto \
  ../proto/*.proto
```

#### 3. Create gRPC Client Service

```dart
// lib/services/fo3_grpc_client.dart
import 'package:grpc/grpc.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../generated/wallet.pbgrpc.dart';
import '../generated/auth.pbgrpc.dart';
import '../generated/earn.pbgrpc.dart';
import '../generated/moonshot.pbgrpc.dart';

class FO3GrpcClient {
  static const String _baseUrl = 'api.fo3wallet.com';
  static const int _port = 50051;
  
  late ClientChannel _channel;
  late AuthServiceClient _authClient;
  late WalletServiceClient _walletClient;
  late EarnServiceClient _earnClient;
  late MoonshotTradingServiceClient _moonshotClient;
  
  String? _accessToken;
  
  FO3GrpcClient() {
    _initializeChannel();
  }
  
  void _initializeChannel() {
    _channel = ClientChannel(
      _baseUrl,
      port: _port,
      options: const ChannelOptions(
        credentials: ChannelCredentials.secure(),
      ),
    );
    
    _authClient = AuthServiceClient(_channel);
    _walletClient = WalletServiceClient(_channel);
    _earnClient = EarnServiceClient(_channel);
    _moonshotClient = MoonshotTradingServiceClient(_channel);
  }
  
  // Authentication
  Future<LoginResponse> login(String email, String password) async {
    final request = LoginRequest()
      ..email = email
      ..password = password;
    
    try {
      final response = await _authClient.login(request);
      _accessToken = response.accessToken;
      
      // Store token securely
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString('access_token', response.accessToken);
      await prefs.setString('refresh_token', response.refreshToken);
      
      return response;
    } catch (e) {
      throw Exception('Login failed: $e');
    }
  }
  
  // Get authenticated call options
  CallOptions _getAuthCallOptions() {
    if (_accessToken == null) {
      throw Exception('Not authenticated');
    }
    
    return CallOptions(
      metadata: {'authorization': 'Bearer $_accessToken'},
    );
  }
  
  // Wallet operations
  Future<GetBalanceResponse> getWalletBalance(String walletId, String currency) async {
    final request = GetBalanceRequest()
      ..walletId = walletId
      ..currency = currency;
    
    return await _walletClient.getBalance(request, options: _getAuthCallOptions());
  }
  
  Future<ListWalletsResponse> listWallets(String userId) async {
    final request = ListWalletsRequest()
      ..userId = userId;
    
    return await _walletClient.listWallets(request, options: _getAuthCallOptions());
  }
  
  // Earn service operations
  Future<GetYieldProductsResponse> getYieldProducts({
    String? category,
    double? minApy,
    String? riskLevel,
  }) async {
    final request = GetYieldProductsRequest();
    if (category != null) request.category = category;
    if (minApy != null) request.minApy = minApy;
    if (riskLevel != null) request.riskLevel = riskLevel;
    
    return await _earnClient.getYieldProducts(request, options: _getAuthCallOptions());
  }
  
  Future<DepositToProductResponse> depositToProduct(
    String userId,
    String productId,
    String amount,
    String currency,
  ) async {
    final request = DepositToProductRequest()
      ..userId = userId
      ..productId = productId
      ..amount = amount
      ..currency = currency;
    
    return await _earnClient.depositToProduct(request, options: _getAuthCallOptions());
  }
  
  // Moonshot trading operations
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
    
    return await _moonshotClient.getTrendingTokens(request, options: _getAuthCallOptions());
  }
  
  Future<VoteOnTokenResponse> voteOnToken(
    String userId,
    String tokenId,
    VoteType voteType,
    int rating,
    String comment,
  ) async {
    final request = VoteOnTokenRequest()
      ..userId = userId
      ..tokenId = tokenId
      ..voteType = voteType
      ..rating = rating
      ..comment = comment;
    
    return await _moonshotClient.voteOnToken(request, options: _getAuthCallOptions());
  }
  
  // Cleanup
  Future<void> shutdown() async {
    await _channel.shutdown();
  }
}
```

#### 4. WebSocket Integration

```dart
// lib/services/fo3_websocket_client.dart
import 'dart:convert';
import 'package:web_socket_channel/web_socket_channel.dart';

class FO3WebSocketClient {
  static const String _wsUrl = 'wss://ws.fo3wallet.com:8080';
  
  WebSocketChannel? _channel;
  Stream<dynamic>? _stream;
  
  void connect(String accessToken) {
    _channel = WebSocketChannel.connect(
      Uri.parse(_wsUrl),
      protocols: ['Bearer $accessToken'],
    );
    
    _stream = _channel!.stream.map((data) => jsonDecode(data));
  }
  
  void subscribe(List<String> events) {
    if (_channel != null) {
      _channel!.sink.add(jsonEncode({
        'type': 'subscribe',
        'events': events,
      }));
    }
  }
  
  Stream<dynamic>? get stream => _stream;
  
  void disconnect() {
    _channel?.sink.close();
  }
}
```

### Swift iOS Setup

#### 1. Add Dependencies

```swift
// Package.swift or Xcode Package Manager
dependencies: [
    .package(url: "https://github.com/grpc/grpc-swift.git", from: "1.15.0"),
    .package(url: "https://github.com/apple/swift-protobuf.git", from: "1.21.0"),
]
```

#### 2. Generate Swift gRPC Code

```bash
# Install Swift protoc plugin
brew install swift-protobuf

# Generate Swift code
mkdir -p Sources/Generated
protoc --swift_out=Sources/Generated \
  --grpc-swift_out=Sources/Generated \
  --proto_path=../proto \
  ../proto/*.proto
```

#### 3. Create gRPC Client Service

```swift
// Sources/Services/FO3GrpcClient.swift
import Foundation
import GRPC
import NIO
import NIOSSL

class FO3GrpcClient {
    private let group: EventLoopGroup
    private let channel: GRPCChannel
    private let authClient: Fo3_Wallet_V1_AuthServiceNIOClient
    private let walletClient: Fo3_Wallet_V1_WalletServiceNIOClient
    private let earnClient: Fo3_Wallet_V1_EarnServiceNIOClient
    private let moonshotClient: Fo3_Wallet_V1_MoonshotTradingServiceNIOClient
    
    private var accessToken: String?
    
    init() {
        self.group = MultiThreadedEventLoopGroup(numberOfThreads: 1)
        
        // Configure TLS
        let tlsConfiguration = TLSConfiguration.makeClientConfiguration()
        let clientTLSProvider = try! NIOSSLContext(configuration: tlsConfiguration)
        
        self.channel = try! GRPCChannelPool.with(
            target: .host("api.fo3wallet.com", port: 50051),
            transportSecurity: .tls(clientTLSProvider),
            eventLoopGroup: group
        )
        
        self.authClient = Fo3_Wallet_V1_AuthServiceNIOClient(channel: channel)
        self.walletClient = Fo3_Wallet_V1_WalletServiceNIOClient(channel: channel)
        self.earnClient = Fo3_Wallet_V1_EarnServiceNIOClient(channel: channel)
        self.moonshotClient = Fo3_Wallet_V1_MoonshotTradingServiceNIOClient(channel: channel)
    }
    
    // Authentication
    func login(email: String, password: String) async throws -> Fo3_Wallet_V1_LoginResponse {
        var request = Fo3_Wallet_V1_LoginRequest()
        request.email = email
        request.password = password
        
        let response = try await authClient.login(request).response.get()
        self.accessToken = response.accessToken
        
        // Store token securely in Keychain
        try storeTokenInKeychain(response.accessToken)
        
        return response
    }
    
    // Get authenticated call options
    private func getAuthCallOptions() -> CallOptions {
        guard let token = accessToken else {
            fatalError("Not authenticated")
        }
        
        var callOptions = CallOptions()
        callOptions.customMetadata.add(name: "authorization", value: "Bearer \(token)")
        return callOptions
    }
    
    // Wallet operations
    func getWalletBalance(walletId: String, currency: String) async throws -> Fo3_Wallet_V1_GetBalanceResponse {
        var request = Fo3_Wallet_V1_GetBalanceRequest()
        request.walletID = walletId
        request.currency = currency
        
        return try await walletClient.getBalance(request, callOptions: getAuthCallOptions()).response.get()
    }
    
    func listWallets(userId: String) async throws -> Fo3_Wallet_V1_ListWalletsResponse {
        var request = Fo3_Wallet_V1_ListWalletsRequest()
        request.userID = userId
        
        return try await walletClient.listWallets(request, callOptions: getAuthCallOptions()).response.get()
    }
    
    // Earn service operations
    func getYieldProducts(category: String? = nil, minApy: Double? = nil, riskLevel: String? = nil) async throws -> Fo3_Wallet_V1_GetYieldProductsResponse {
        var request = Fo3_Wallet_V1_GetYieldProductsRequest()
        if let category = category { request.category = category }
        if let minApy = minApy { request.minApy = minApy }
        if let riskLevel = riskLevel { request.riskLevel = riskLevel }
        
        return try await earnClient.getYieldProducts(request, callOptions: getAuthCallOptions()).response.get()
    }
    
    func depositToProduct(userId: String, productId: String, amount: String, currency: String) async throws -> Fo3_Wallet_V1_DepositToProductResponse {
        var request = Fo3_Wallet_V1_DepositToProductRequest()
        request.userID = userId
        request.productID = productId
        request.amount = amount
        request.currency = currency
        
        return try await earnClient.depositToProduct(request, callOptions: getAuthCallOptions()).response.get()
    }
    
    // Moonshot trading operations
    func getTrendingTokens(page: Int32 = 1, pageSize: Int32 = 20, timeFrame: String? = nil, sortBy: String? = nil) async throws -> Fo3_Wallet_V1_GetTrendingTokensResponse {
        var request = Fo3_Wallet_V1_GetTrendingTokensRequest()
        request.page = page
        request.pageSize = pageSize
        if let timeFrame = timeFrame { request.timeFrame = timeFrame }
        if let sortBy = sortBy { request.sortBy = sortBy }
        
        return try await moonshotClient.getTrendingTokens(request, callOptions: getAuthCallOptions()).response.get()
    }
    
    func voteOnToken(userId: String, tokenId: String, voteType: Fo3_Wallet_V1_VoteType, rating: Int32, comment: String) async throws -> Fo3_Wallet_V1_VoteOnTokenResponse {
        var request = Fo3_Wallet_V1_VoteOnTokenRequest()
        request.userID = userId
        request.tokenID = tokenId
        request.voteType = voteType
        request.rating = rating
        request.comment = comment
        
        return try await moonshotClient.voteOnToken(request, callOptions: getAuthCallOptions()).response.get()
    }
    
    // Keychain storage
    private func storeTokenInKeychain(_ token: String) throws {
        let data = token.data(using: .utf8)!
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: "fo3_access_token",
            kSecValueData as String: data
        ]
        
        SecItemDelete(query as CFDictionary)
        let status = SecItemAdd(query as CFDictionary, nil)
        
        if status != errSecSuccess {
            throw NSError(domain: "KeychainError", code: Int(status), userInfo: nil)
        }
    }
    
    // Cleanup
    func shutdown() {
        try? channel.close().wait()
        try? group.syncShutdownGracefully()
    }
}
```

## 🔄 Real-time Updates

### Flutter WebSocket Example

```dart
// lib/widgets/live_price_widget.dart
import 'package:flutter/material.dart';
import '../services/fo3_websocket_client.dart';

class LivePriceWidget extends StatefulWidget {
  @override
  _LivePriceWidgetState createState() => _LivePriceWidgetState();
}

class _LivePriceWidgetState extends State<LivePriceWidget> {
  final FO3WebSocketClient _wsClient = FO3WebSocketClient();
  Map<String, dynamic> _priceData = {};
  
  @override
  void initState() {
    super.initState();
    _connectWebSocket();
  }
  
  void _connectWebSocket() {
    _wsClient.connect('your_access_token');
    _wsClient.subscribe(['price_update', 'yield_update']);
    
    _wsClient.stream?.listen((data) {
      if (data['type'] == 'price_update') {
        setState(() {
          _priceData = data['payload'];
        });
      }
    });
  }
  
  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: EdgeInsets.all(16),
        child: Column(
          children: [
            Text('Live Prices', style: Theme.of(context).textTheme.headline6),
            ..._priceData.entries.map((entry) => 
              ListTile(
                title: Text(entry.key),
                trailing: Text('\$${entry.value}'),
              )
            ).toList(),
          ],
        ),
      ),
    );
  }
  
  @override
  void dispose() {
    _wsClient.disconnect();
    super.dispose();
  }
}
```

## 🔐 Security Best Practices

### Token Management

1. **Secure Storage:**
   - Flutter: Use `flutter_secure_storage` package
   - iOS: Store tokens in iOS Keychain

2. **Token Refresh:**
   - Implement automatic token refresh
   - Handle authentication errors gracefully

3. **Certificate Pinning:**
   - Pin SSL certificates for production
   - Validate server certificates

### Error Handling

```dart
// lib/utils/grpc_error_handler.dart
import 'package:grpc/grpc.dart';

class GrpcErrorHandler {
  static String handleError(GrpcError error) {
    switch (error.code) {
      case StatusCode.unauthenticated:
        return 'Authentication required. Please log in.';
      case StatusCode.permissionDenied:
        return 'Permission denied. Check your access rights.';
      case StatusCode.resourceExhausted:
        return 'Rate limit exceeded. Please try again later.';
      case StatusCode.unavailable:
        return 'Service temporarily unavailable.';
      default:
        return 'An error occurred: ${error.message}';
    }
  }
}
```

## 📱 UI Integration Examples

### Flutter Yield Products Screen

```dart
// lib/screens/yield_products_screen.dart
import 'package:flutter/material.dart';
import '../services/fo3_grpc_client.dart';
import '../generated/earn.pb.dart';

class YieldProductsScreen extends StatefulWidget {
  @override
  _YieldProductsScreenState createState() => _YieldProductsScreenState();
}

class _YieldProductsScreenState extends State<YieldProductsScreen> {
  final FO3GrpcClient _grpcClient = FO3GrpcClient();
  List<YieldProduct> _products = [];
  bool _loading = true;
  
  @override
  void initState() {
    super.initState();
    _loadYieldProducts();
  }
  
  Future<void> _loadYieldProducts() async {
    try {
      final response = await _grpcClient.getYieldProducts();
      setState(() {
        _products = response.products;
        _loading = false;
      });
    } catch (e) {
      setState(() {
        _loading = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Failed to load products: $e')),
      );
    }
  }
  
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text('Yield Products')),
      body: _loading
          ? Center(child: CircularProgressIndicator())
          : ListView.builder(
              itemCount: _products.length,
              itemBuilder: (context, index) {
                final product = _products[index];
                return Card(
                  margin: EdgeInsets.all(8),
                  child: ListTile(
                    title: Text(product.name),
                    subtitle: Text(product.description),
                    trailing: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Text('${product.apy}% APY'),
                        Text(product.riskLevel),
                      ],
                    ),
                    onTap: () => _showProductDetails(product),
                  ),
                );
              },
            ),
    );
  }
  
  void _showProductDetails(YieldProduct product) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(product.name),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('APY: ${product.apy}%'),
            Text('Risk Level: ${product.riskLevel}'),
            Text('Min Deposit: ${product.minDeposit} ${product.currency}'),
            Text('TVL: \$${product.tvl}'),
            SizedBox(height: 16),
            Text(product.description),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: Text('Close'),
          ),
          ElevatedButton(
            onPressed: () => _depositToProduct(product),
            child: Text('Deposit'),
          ),
        ],
      ),
    );
  }
  
  void _depositToProduct(YieldProduct product) {
    // Implement deposit logic
    Navigator.pop(context);
    // Navigate to deposit screen
  }
}
```

## 🧪 Testing

### Unit Tests

```dart
// test/services/fo3_grpc_client_test.dart
import 'package:flutter_test/flutter_test.dart';
import 'package:mockito/mockito.dart';
import '../lib/services/fo3_grpc_client.dart';

void main() {
  group('FO3GrpcClient', () {
    late FO3GrpcClient client;
    
    setUp(() {
      client = FO3GrpcClient();
    });
    
    test('should login successfully', () async {
      // Mock login test
      final response = await client.login('test@example.com', 'password');
      expect(response.accessToken, isNotEmpty);
    });
    
    test('should get wallet balance', () async {
      // Mock balance test
      await client.login('test@example.com', 'password');
      final balance = await client.getWalletBalance('wallet-123', 'ETH');
      expect(balance.balance, isNotEmpty);
    });
  });
}
```

## 📚 Additional Resources

- [gRPC Dart Documentation](https://grpc.io/docs/languages/dart/)
- [gRPC Swift Documentation](https://grpc.io/docs/languages/swift/)
- [Protocol Buffers Guide](https://developers.google.com/protocol-buffers)
- [FO3 Wallet Core API Reference](./API_REFERENCE.md)

## 🆘 Support

For mobile integration support:
- GitHub Issues: [fo3-wallet-core/issues](https://github.com/bankchen1/fo3-wallet-core/issues)
- Mobile Team Slack: #mobile-integration
- Email: mobile-support@fo3wallet.com
