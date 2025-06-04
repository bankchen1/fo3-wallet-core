/// FO3 Wallet SDK Flutter Example
/// 
/// Demonstrates comprehensive usage of the FO3 Wallet SDK including:
/// - Authentication and user management
/// - Real-time market data and notifications
/// - Yield farming and DeFi operations
/// - Moonshot token discovery and voting
/// - Portfolio management and analytics
/// - Card management and transactions

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:fo3_wallet_sdk/fo3_wallet_sdk.dart';

void main() {
  runApp(const FO3WalletExampleApp());
}

class FO3WalletExampleApp extends StatelessWidget {
  const FO3WalletExampleApp({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return ChangeNotifierProvider(
      create: (context) => AppState(),
      child: MaterialApp(
        title: 'FO3 Wallet SDK Example',
        theme: ThemeData(
          primarySwatch: Colors.blue,
          useMaterial3: true,
        ),
        home: const SplashScreen(),
      ),
    );
  }
}

class AppState extends ChangeNotifier {
  FO3WalletSDK? _sdk;
  AuthState _authState = AuthState.unauthenticated;
  String? _error;
  List<YieldProduct> _yieldProducts = [];
  List<Token> _trendingTokens = [];
  Map<String, dynamic>? _marketData;

  // Getters
  FO3WalletSDK? get sdk => _sdk;
  AuthState get authState => _authState;
  String? get error => _error;
  List<YieldProduct> get yieldProducts => _yieldProducts;
  List<Token> get trendingTokens => _trendingTokens;
  Map<String, dynamic>? get marketData => _marketData;
  bool get isAuthenticated => _authState == AuthState.authenticated;

  Future<void> initializeSDK() async {
    try {
      _sdk = await FO3WalletSDK.initialize(FO3WalletConfig.development);
      
      // Listen to auth state changes
      _sdk!.authStateStream.listen((state) {
        _authState = state;
        notifyListeners();
      });

      // Listen to real-time notifications
      _sdk!.notificationStream.listen((notification) {
        _handleNotification(notification);
      });

      notifyListeners();
    } catch (e) {
      _error = e.toString();
      notifyListeners();
    }
  }

  Future<void> login(String email, String password) async {
    if (_sdk == null) return;

    try {
      _error = null;
      await _sdk!.login(email, password);
      
      // Load initial data after successful login
      await _loadInitialData();
    } catch (e) {
      _error = e.toString();
      notifyListeners();
    }
  }

  Future<void> logout() async {
    if (_sdk == null) return;

    try {
      await _sdk!.logout();
      _yieldProducts.clear();
      _trendingTokens.clear();
      _marketData = null;
      notifyListeners();
    } catch (e) {
      _error = e.toString();
      notifyListeners();
    }
  }

  Future<void> _loadInitialData() async {
    if (_sdk == null || !isAuthenticated) return;

    try {
      // Load yield products
      final yieldResponse = await _sdk!.getYieldProducts();
      _yieldProducts = yieldResponse.products;

      // Load trending tokens
      final trendingResponse = await _sdk!.getTrendingTokens();
      _trendingTokens = trendingResponse.tokens;

      // Load market data
      final marketResponse = await _sdk!.getRealTimeMarketData(
        ['ETH', 'BTC', 'USDC'],
        includeOrderbook: true,
        includeTrades: true,
      );
      _marketData = {
        'dataPoints': marketResponse.dataPoints,
        'summary': marketResponse.marketSummary,
      };

      notifyListeners();
    } catch (e) {
      _error = e.toString();
      notifyListeners();
    }
  }

  void _handleNotification(Map<String, dynamic> notification) {
    // Handle real-time notifications
    switch (notification['type']) {
      case 'price_update':
        // Update market data
        break;
      case 'yield_update':
        // Update yield products
        break;
      case 'transaction_confirmed':
        // Handle transaction confirmation
        break;
    }
    notifyListeners();
  }

  void clearError() {
    _error = null;
    notifyListeners();
  }
}

class SplashScreen extends StatefulWidget {
  const SplashScreen({Key? key}) : super(key: key);

  @override
  State<SplashScreen> createState() => _SplashScreenState();
}

class _SplashScreenState extends State<SplashScreen> {
  @override
  void initState() {
    super.initState();
    _initializeApp();
  }

  Future<void> _initializeApp() async {
    final appState = Provider.of<AppState>(context, listen: false);
    await appState.initializeSDK();
    
    // Navigate to appropriate screen
    if (mounted) {
      Navigator.of(context).pushReplacement(
        MaterialPageRoute(
          builder: (context) => Consumer<AppState>(
            builder: (context, appState, child) {
              if (appState.isAuthenticated) {
                return const DashboardScreen();
              } else {
                return const LoginScreen();
              }
            },
          ),
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Image.asset('assets/logo.png', width: 120, height: 120),
            const SizedBox(height: 24),
            const Text(
              'FO3 Wallet',
              style: TextStyle(fontSize: 32, fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 16),
            const CircularProgressIndicator(),
            const SizedBox(height: 16),
            const Text('Initializing SDK...'),
          ],
        ),
      ),
    );
  }
}

class LoginScreen extends StatefulWidget {
  const LoginScreen({Key? key}) : super(key: key);

  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  final _emailController = TextEditingController();
  final _passwordController = TextEditingController();
  bool _isLoading = false;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Login'),
      ),
      body: Consumer<AppState>(
        builder: (context, appState, child) {
          if (appState.isAuthenticated) {
            WidgetsBinding.instance.addPostFrameCallback((_) {
              Navigator.of(context).pushReplacement(
                MaterialPageRoute(builder: (context) => const DashboardScreen()),
              );
            });
          }

          return Padding(
            padding: const EdgeInsets.all(16.0),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                if (appState.error != null) ...[
                  Card(
                    color: Colors.red.shade50,
                    child: Padding(
                      padding: const EdgeInsets.all(16.0),
                      child: Row(
                        children: [
                          Icon(Icons.error, color: Colors.red.shade700),
                          const SizedBox(width: 8),
                          Expanded(
                            child: Text(
                              appState.error!,
                              style: TextStyle(color: Colors.red.shade700),
                            ),
                          ),
                          IconButton(
                            onPressed: appState.clearError,
                            icon: const Icon(Icons.close),
                          ),
                        ],
                      ),
                    ),
                  ),
                  const SizedBox(height: 16),
                ],
                TextField(
                  controller: _emailController,
                  decoration: const InputDecoration(
                    labelText: 'Email',
                    border: OutlineInputBorder(),
                  ),
                  keyboardType: TextInputType.emailAddress,
                ),
                const SizedBox(height: 16),
                TextField(
                  controller: _passwordController,
                  decoration: const InputDecoration(
                    labelText: 'Password',
                    border: OutlineInputBorder(),
                  ),
                  obscureText: true,
                ),
                const SizedBox(height: 24),
                SizedBox(
                  width: double.infinity,
                  child: ElevatedButton(
                    onPressed: _isLoading ? null : _login,
                    child: _isLoading
                        ? const CircularProgressIndicator()
                        : const Text('Login'),
                  ),
                ),
              ],
            ),
          );
        },
      ),
    );
  }

  Future<void> _login() async {
    if (_emailController.text.isEmpty || _passwordController.text.isEmpty) {
      return;
    }

    setState(() {
      _isLoading = true;
    });

    final appState = Provider.of<AppState>(context, listen: false);
    await appState.login(_emailController.text, _passwordController.text);

    setState(() {
      _isLoading = false;
    });
  }

  @override
  void dispose() {
    _emailController.dispose();
    _passwordController.dispose();
    super.dispose();
  }
}

class DashboardScreen extends StatefulWidget {
  const DashboardScreen({Key? key}) : super(key: key);

  @override
  State<DashboardScreen> createState() => _DashboardScreenState();
}

class _DashboardScreenState extends State<DashboardScreen> {
  int _selectedIndex = 0;

  final List<Widget> _screens = [
    const PortfolioScreen(),
    const YieldFarmingScreen(),
    const MoonshotScreen(),
    const MarketDataScreen(),
  ];

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('FO3 Wallet'),
        actions: [
          Consumer<AppState>(
            builder: (context, appState, child) {
              return IconButton(
                onPressed: appState.logout,
                icon: const Icon(Icons.logout),
              );
            },
          ),
        ],
      ),
      body: _screens[_selectedIndex],
      bottomNavigationBar: BottomNavigationBar(
        type: BottomNavigationBarType.fixed,
        currentIndex: _selectedIndex,
        onTap: (index) {
          setState(() {
            _selectedIndex = index;
          });
        },
        items: const [
          BottomNavigationBarItem(
            icon: Icon(Icons.account_balance_wallet),
            label: 'Portfolio',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.agriculture),
            label: 'Yield',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.rocket_launch),
            label: 'Moonshot',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.analytics),
            label: 'Market',
          ),
        ],
      ),
    );
  }
}

class PortfolioScreen extends StatelessWidget {
  const PortfolioScreen({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return const Center(
      child: Text('Portfolio Screen - Coming Soon'),
    );
  }
}

class YieldFarmingScreen extends StatelessWidget {
  const YieldFarmingScreen({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Consumer<AppState>(
      builder: (context, appState, child) {
        if (appState.yieldProducts.isEmpty) {
          return const Center(
            child: CircularProgressIndicator(),
          );
        }

        return ListView.builder(
          itemCount: appState.yieldProducts.length,
          itemBuilder: (context, index) {
            final product = appState.yieldProducts[index];
            return Card(
              margin: const EdgeInsets.all(8.0),
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
                onTap: () {
                  // Navigate to product details
                },
              ),
            );
          },
        );
      },
    );
  }
}

class MoonshotScreen extends StatelessWidget {
  const MoonshotScreen({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Consumer<AppState>(
      builder: (context, appState, child) {
        if (appState.trendingTokens.isEmpty) {
          return const Center(
            child: CircularProgressIndicator(),
          );
        }

        return ListView.builder(
          itemCount: appState.trendingTokens.length,
          itemBuilder: (context, index) {
            final token = appState.trendingTokens[index];
            return Card(
              margin: const EdgeInsets.all(8.0),
              child: ListTile(
                leading: CircleAvatar(
                  backgroundImage: NetworkImage(token.logoUrl),
                ),
                title: Text('${token.name} (${token.symbol})'),
                subtitle: Text(token.description),
                trailing: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Text('\$${token.metrics?.currentPrice ?? '0.00'}'),
                    Text(
                      '${token.metrics?.priceChangePercentage24h ?? '0.00'}%',
                      style: TextStyle(
                        color: (token.metrics?.priceChangePercentage24h?.startsWith('-') ?? false)
                            ? Colors.red
                            : Colors.green,
                      ),
                    ),
                  ],
                ),
                onTap: () {
                  // Navigate to token details
                },
              ),
            );
          },
        );
      },
    );
  }
}

class MarketDataScreen extends StatelessWidget {
  const MarketDataScreen({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Consumer<AppState>(
      builder: (context, appState, child) {
        final marketData = appState.marketData;
        if (marketData == null) {
          return const Center(
            child: CircularProgressIndicator(),
          );
        }

        final dataPoints = marketData['dataPoints'] as List<MarketDataPoint>;
        final summary = marketData['summary'] as MarketSummary?;

        return Column(
          children: [
            if (summary != null) ...[
              Card(
                margin: const EdgeInsets.all(8.0),
                child: Padding(
                  padding: const EdgeInsets.all(16.0),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text(
                        'Market Summary',
                        style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
                      ),
                      const SizedBox(height: 8),
                      Text('Total Market Cap: \$${summary.totalMarketCap}'),
                      Text('24h Volume: \$${summary.totalVolume24h}'),
                      Text('BTC Dominance: ${summary.btcDominance}%'),
                      Text('Fear & Greed Index: ${summary.fearGreedIndex}'),
                    ],
                  ),
                ),
              ),
            ],
            Expanded(
              child: ListView.builder(
                itemCount: dataPoints.length,
                itemBuilder: (context, index) {
                  final dataPoint = dataPoints[index];
                  return Card(
                    margin: const EdgeInsets.all(8.0),
                    child: ListTile(
                      title: Text(dataPoint.symbol),
                      subtitle: Text('${dataPoint.blockchain} • \$${dataPoint.price}'),
                      trailing: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          Text('Vol: \$${dataPoint.volume24h}'),
                          Text(
                            '${dataPoint.priceChangePercentage24h}%',
                            style: TextStyle(
                              color: dataPoint.priceChangePercentage24h.startsWith('-')
                                  ? Colors.red
                                  : Colors.green,
                            ),
                          ),
                        ],
                      ),
                    ),
                  );
                },
              ),
            ),
          ],
        );
      },
    );
  }
}
