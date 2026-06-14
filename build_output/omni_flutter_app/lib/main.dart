import 'package:flutter/material.dart';
import 'blog_dashboard.dart';

void main() => runApp(const OmniTestApp());

class OmniTestApp extends StatelessWidget {
  const OmniTestApp({super.key});

  @override
  Widget build(BuildContext context) {
    return const MaterialApp(
      debugShowCheckedModeBanner: false,
      home: Scaffold(
        body: Center(
          child: BlogDashboard(),
        ),
      ),
    );
  }
}
