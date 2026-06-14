import 'package:flutter/material.dart';

// .omni:1 -- component Counter
class Counter extends StatefulWidget {
  const Counter({super.key});
  @override
  State<Counter> createState() => _CounterState();
}

class _CounterState extends State<Counter> {
  int _count = 0;

  void _increment() {
    setState(() {
      _count = (_count + 1);
    });
  }

  @override
  Widget build(BuildContext context) {
    return Container(child: Column(children: [Text(style: TextStyle(fontSize: 28, fontWeight: FontWeight.w600), 'Count: ${_count}', key: const ValueKey("counter-label")), ElevatedButton(style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF4338CA), foregroundColor: Colors.white, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _increment(), child: Text("+"), key: const ValueKey("btn-increment"))]));
  }
}