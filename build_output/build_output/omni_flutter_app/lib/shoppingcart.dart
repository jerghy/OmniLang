import 'package:flutter/material.dart';

// .omni:1 -- component ShoppingCart
class ShoppingCart extends StatefulWidget {
  const ShoppingCart({super.key});
  @override
  State<ShoppingCart> createState() => _ShoppingCartState();
}

class _ShoppingCartState extends State<ShoppingCart> {
  int _count = 0;
  int _total = 0;

  void _addApple() {
    setState(() {
      _count = (_count + 1);
      _total = (_total + 5);
    });
  }

  void _addBanana() {
    setState(() {
      _count = (_count + 1);
      _total = (_total + 3);
    });
  }

  void _addOrange() {
    setState(() {
      _count = (_count + 1);
      _total = (_total + 4);
    });
  }

  void _clearCart() {
    setState(() {
      _count = 0;
      _total = 0;
    });
  }

  @override
  Widget build(BuildContext context) {
    return Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), child: Padding(padding: const EdgeInsets.all(24), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [Text(style: TextStyle(fontSize: 24, fontWeight: FontWeight.w600), "Shopping Cart", key: const ValueKey("cart-title")), Text('Items: ${_count}', key: const ValueKey("cart-items")), Text('Total: \$${_total}', key: const ValueKey("cart-total")), Row(spacing: 8, key: const ValueKey("product-buttons"), children: [ElevatedButton(style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF4F46E5), foregroundColor: Colors.white, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _addApple(), child: Text('Apple \$5'), key: const ValueKey("btn-apple")), ElevatedButton(style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF4F46E5), foregroundColor: Colors.white, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _addBanana(), child: Text('Banana \$3'), key: const ValueKey("btn-banana")), ElevatedButton(style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF4F46E5), foregroundColor: Colors.white, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _addOrange(), child: Text('Orange \$4'), key: const ValueKey("btn-orange"))]), OutlinedButton(style: OutlinedButton.styleFrom(backgroundColor: Colors.transparent, foregroundColor: const Color(0xFF374151), side: const BorderSide(color: Color(0xFFE5E7EB)), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _clearCart(), child: Text("Clear Cart"), key: const ValueKey("btn-clear"))])), key: const ValueKey("cart-root"));
  }
}