import 'package:flutter/material.dart';

// .omni:1 -- component BlogDashboard
class BlogDashboard extends StatefulWidget {
  const BlogDashboard({super.key});
  @override
  State<BlogDashboard> createState() => _BlogDashboardState();
}

class _BlogDashboardState extends State<BlogDashboard> {
  String _activeMenu = "articles";
  int _totalArticles = 128;
  int _totalViews = 45678;
  int _totalComments = 1234;
  int _totalLikes = 9876;
  String _searchQuery = "";
  String _activeCategory = "all";

  void _selectDashboard() {
    setState(() {
      _activeMenu = "dashboard";
    });
  }

  void _selectArticles() {
    setState(() {
      _activeMenu = "articles";
    });
  }

  void _selectTheme() {
    setState(() {
      _activeMenu = "theme";
    });
  }

  void _selectStats() {
    setState(() {
      _activeMenu = "stats";
    });
  }

  void _filterAll() {
    setState(() {
      _activeCategory = "all";
    });
  }

  void _filterTech() {
    setState(() {
      _activeCategory = "tech";
    });
  }

  void _filterDesign() {
    setState(() {
      _activeCategory = "design";
    });
  }

  void _filterBusiness() {
    setState(() {
      _activeCategory = "business";
    });
  }

  @override
  Widget build(BuildContext context) {
    return Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("dashboard-root"), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [Text(style: TextStyle(fontSize: 28, fontWeight: FontWeight.w600), "Blog Dashboard", key: const ValueKey("app-title")), Row(spacing: 24, key: const ValueKey("main-layout"), children: [Column(spacing: 4, crossAxisAlignment: CrossAxisAlignment.start, key: const ValueKey("sidebar"), children: [OutlinedButton(style: OutlinedButton.styleFrom(foregroundColor: const Color(0xFF374151), side: const BorderSide(color: Color(0xFFE5E7EB)), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _selectDashboard(), child: Text("Dashboard"), key: const ValueKey("nav-dashboard")), ElevatedButton(style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF4338CA), foregroundColor: Colors.white, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _selectArticles(), child: Text("Articles"), key: const ValueKey("nav-articles")), OutlinedButton(style: OutlinedButton.styleFrom(foregroundColor: const Color(0xFF374151), side: const BorderSide(color: Color(0xFFE5E7EB)), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _selectTheme(), child: Text("Theme Settings"), key: const ValueKey("nav-theme")), OutlinedButton(style: OutlinedButton.styleFrom(foregroundColor: const Color(0xFF374151), side: const BorderSide(color: Color(0xFFE5E7EB)), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _selectStats(), child: Text("Statistics"), key: const ValueKey("nav-stats"))]), Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("main-content"), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [Row(spacing: 16, key: const ValueKey("kpi-row"), children: [Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("kpi-articles"), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [Text('${_totalArticles}', key: const ValueKey("kpi-articles-value")), Text("Total Articles", key: const ValueKey("kpi-articles-label"))])), Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("kpi-views"), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [Text('${_totalViews}', key: const ValueKey("kpi-views-value")), Text("Total Views", key: const ValueKey("kpi-views-label"))])), Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("kpi-comments"), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [Text('${_totalComments}', key: const ValueKey("kpi-comments-value")), Text("Comments", key: const ValueKey("kpi-comments-label"))])), Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("kpi-likes"), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [Text('${_totalLikes}', key: const ValueKey("kpi-likes-value")), Text("Likes", key: const ValueKey("kpi-likes-label"))]))]), Row(spacing: 12, key: const ValueKey("search-row"), children: [TextField(decoration: const InputDecoration(hintText: "Search articles..."), key: const ValueKey("search-input")), ElevatedButton(style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF4338CA), foregroundColor: Colors.white, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: null, child: Text("Search"), key: const ValueKey("btn-search"))]), Row(spacing: 8, key: const ValueKey("category-filters"), children: [Text("Category:"), ElevatedButton(style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF4338CA), foregroundColor: Colors.white, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _filterAll(), child: Text("All"), key: const ValueKey("cat-all")), OutlinedButton(style: OutlinedButton.styleFrom(foregroundColor: const Color(0xFF374151), side: const BorderSide(color: Color(0xFFE5E7EB)), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _filterTech(), child: Text("Tech"), key: const ValueKey("cat-tech")), OutlinedButton(style: OutlinedButton.styleFrom(foregroundColor: const Color(0xFF374151), side: const BorderSide(color: Color(0xFFE5E7EB)), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _filterDesign(), child: Text("Design"), key: const ValueKey("cat-design")), OutlinedButton(style: OutlinedButton.styleFrom(foregroundColor: const Color(0xFF374151), side: const BorderSide(color: Color(0xFFE5E7EB)), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _filterBusiness(), child: Text("Business"), key: const ValueKey("cat-business"))]), Column(spacing: 16, crossAxisAlignment: CrossAxisAlignment.start, key: const ValueKey("article-list"), children: [Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("article-1"), child: Row(spacing: 0, children: [Container(key: const ValueKey("article-1-cover")), Column(spacing: 8, crossAxisAlignment: CrossAxisAlignment.start, children: [Text("Technology", key: const ValueKey("article-1-category")), Text(style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600), "Building Scalable Microservices with Rust", key: const ValueKey("article-1-title")), Text("Learn how to design and deploy high-performance microservices using Rust's async runtime and zero-cost abstractions.", key: const ValueKey("article-1-excerpt")), Row(spacing: 10, mainAxisAlignment: MainAxisAlignment.center, crossAxisAlignment: CrossAxisAlignment.center, children: [Container(key: const ValueKey("article-1-avatar")), Text("Alice Chen", key: const ValueKey("article-1-author")), Text("Jun 12, 2026", key: const ValueKey("article-1-date"))])])])), Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("article-2"), child: Row(spacing: 0, children: [Container(key: const ValueKey("article-2-cover")), Column(spacing: 8, crossAxisAlignment: CrossAxisAlignment.start, children: [Text("Design", key: const ValueKey("article-2-category")), Text(style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600), "The Future of UI: Beyond Flat Design", key: const ValueKey("article-2-title")), Text("Exploring neumorphism, glassmorphism, and the next generation of user interface design patterns.", key: const ValueKey("article-2-excerpt")), Row(spacing: 10, mainAxisAlignment: MainAxisAlignment.center, crossAxisAlignment: CrossAxisAlignment.center, children: [Container(key: const ValueKey("article-2-avatar")), Text("Bob Wang", key: const ValueKey("article-2-author")), Text("Jun 10, 2026", key: const ValueKey("article-2-date"))])])])), Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("article-3"), child: Row(spacing: 0, children: [Container(key: const ValueKey("article-3-cover")), Column(spacing: 8, crossAxisAlignment: CrossAxisAlignment.start, children: [Text("Business", key: const ValueKey("article-3-category")), Text(style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600), "Monetization Strategies for Open Source", key: const ValueKey("article-3-title")), Text("A comprehensive guide to sustainable business models for open-source projects in 2026.", key: const ValueKey("article-3-excerpt")), Row(spacing: 10, mainAxisAlignment: MainAxisAlignment.center, crossAxisAlignment: CrossAxisAlignment.center, children: [Container(key: const ValueKey("article-3-avatar")), Text("Carol Liu", key: const ValueKey("article-3-author")), Text("Jun 8, 2026", key: const ValueKey("article-3-date"))])])]))])]))])]));
  }
}