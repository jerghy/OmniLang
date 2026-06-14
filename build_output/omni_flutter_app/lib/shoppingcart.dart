import 'package:flutter/material.dart';

// .omni:1 -- component ShoppingCart
class ShoppingCart extends StatefulWidget {
  const ShoppingCart({super.key});
  @override
  State<ShoppingCart> createState() => _ShoppingCartState();
}

class _ShoppingCartState extends State<ShoppingCart> {
  int _cartCount = 0;
  int _cartTotal = 0;
  String _lastAdded = "";

  void _addApple() {
    setState(() {
      _cartCount = (_cartCount + 1);
      _cartTotal = (_cartTotal + 5);
      _lastAdded = "Apple (\$5)";
    });
  }

  void _addBanana() {
    setState(() {
      _cartCount = (_cartCount + 1);
      _cartTotal = (_cartTotal + 3);
      _lastAdded = "Banana (\$3)";
    });
  }

  void _addOrange() {
    setState(() {
      _cartCount = (_cartCount + 1);
      _cartTotal = (_cartTotal + 4);
      _lastAdded = "Orange (\$4)";
    });
  }

  void _clearCart() {
    setState(() {
      _cartCount = 0;
      _cartTotal = 0;
      _lastAdded = "";
    });
  }

  @override
  Widget build(BuildContext context) {
    return Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), child: Padding(padding: const EdgeInsets.all(24), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [Text(style: TextStyle(fontSize: 20, fontWeight: FontWeight.w600), "Shopping Cart", key: const ValueKey("cart-title")), Text(style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600), "Products"), Row(spacing: 12, children: [Text('Apple - \$5'), ElevatedButton(style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF4F46E5), foregroundColor: Colors.white, textStyle: const TextStyle(fontSize: 13), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _addApple(), child: Text("Add to Cart"), key: const ValueKey("btn-add-apple"))]), Row(spacing: 12, children: [Text('Banana - \$3'), ElevatedButton(style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF4F46E5), foregroundColor: Colors.white, textStyle: const TextStyle(fontSize: 13), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _addBanana(), child: Text("Add to Cart"), key: const ValueKey("btn-add-banana"))]), Row(spacing: 12, children: [Text('Orange - \$4'), ElevatedButton(style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF4F46E5), foregroundColor: Colors.white, textStyle: const TextStyle(fontSize: 13), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _addOrange(), child: Text("Add to Cart"), key: const ValueKey("btn-add-orange"))]), Text(style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600), "Your Cart", key: const ValueKey("cart-section")), Text('Items: ${_cartCount}', key: const ValueKey("cart-count")), Text('Total: \$${_cartTotal}', key: const ValueKey("cart-total")), Text('${_lastAdded}', key: const ValueKey("last-added")), OutlinedButton(style: OutlinedButton.styleFrom(backgroundColor: Colors.transparent, foregroundColor: const Color(0xFF1F2937), textStyle: const TextStyle(fontSize: 13), side: const BorderSide(color: Color(0xFFE5E7EB)), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _clearCart(), child: Text("Clear Cart"), key: const ValueKey("btn-clear"))])));
  }
}