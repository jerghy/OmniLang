import 'package:flutter/material.dart';

// .omni:1 -- component TaskBoard
class TaskBoard extends StatefulWidget {
  const TaskBoard({super.key});
  @override
  State<TaskBoard> createState() => _TaskBoardState();
}

class _TaskBoardState extends State<TaskBoard> {
  int _aliceTasks = 0;
  int _bobTasks = 0;
  int _charlieTasks = 1;
  String _task1Assignee = "";
  String _task2Assignee = "";
  String _filterPriority = "all";
  bool _isSaving = false;
  String _saveError = "";
  String _errorMessage = "";
  int _maxCapacity = 3;

  void _assignTask1ToAlice() {
    setState(() {
      if ((_aliceTasks < _maxCapacity)) {
          _task1Assignee = "Alice";
          _aliceTasks = (_aliceTasks + 1);
          _errorMessage = "";
      } else {
          _errorMessage = "Alice is at capacity!";
      }
    });
  }

  void _assignTask2ToBob() {
    setState(() {
      if ((_bobTasks < _maxCapacity)) {
          _task2Assignee = "Bob";
          _bobTasks = (_bobTasks + 1);
          _errorMessage = "";
      } else {
          _errorMessage = "Bob is at capacity!";
      }
    });
  }

  void _filterAll() {
    setState(() {
      _filterPriority = "all";
    });
  }

  void _filterHigh() {
    setState(() {
      _filterPriority = "high";
    });
  }

  void _syncToBackend() {
    setState(() {
      if (_isSaving) {
          _isSaving = false;
          _saveError = "Sync failed: network error";
      } else {
          _isSaving = true;
          _saveError = "";
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("task-board-root"), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [Text(style: TextStyle(fontSize: 20, fontWeight: FontWeight.w600), "Task Assignment Dashboard", key: const ValueKey("app-title")), Row(spacing: 16, key: const ValueKey("main-layout"), children: [Column(spacing: 12, crossAxisAlignment: CrossAxisAlignment.start, key: const ValueKey("members-panel"), children: [Text(style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600), "Team Members"), Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("member-alice"), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [Text(style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600), "Alice Chen"), Text('Tasks: ${_aliceTasks} | Done: 0', key: const ValueKey("alice-tasks"))])), Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("member-bob"), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [Text(style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600), "Bob Wang"), Text('Tasks: ${_bobTasks} | Done: 0', key: const ValueKey("bob-tasks"))])), Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("member-charlie"), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [Text(style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600), "Charlie Li"), Text('Tasks: ${_charlieTasks} | Done: 0', key: const ValueKey("charlie-tasks"))]))]), Column(spacing: 12, crossAxisAlignment: CrossAxisAlignment.start, key: const ValueKey("task-panel"), children: [Text(style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600), "Task Board", key: const ValueKey("task-board-title")), Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("task-1"), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [Text(style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600), "T1: Design API"), Text("Status: todo | Priority: high"), Text('Assignee: ${_task1Assignee}', key: const ValueKey("task1-assignee"))])), Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("task-2"), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [Text(style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600), "T2: Build Auth"), Text("Status: todo | Priority: medium"), Text('Assignee: ${_task2Assignee}', key: const ValueKey("task2-assignee"))])), Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("task-3"), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [Text(style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600), "T3: Write Tests"), Text("Status: todo | Priority: low"), Text("Assignee: —")])), Card(color: Colors.white, elevation: 2, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)), key: const ValueKey("task-4"), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [Text(style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600), "T4: Deploy CI"), Text("Status: todo | Priority: high"), Text("Assignee: —")])), Column(spacing: 8, crossAxisAlignment: CrossAxisAlignment.start, key: const ValueKey("assign-buttons"), children: [ElevatedButton(style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF4338CA), foregroundColor: Colors.white, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _assignTask1ToAlice(), child: Text("Assign T1 to Alice"), key: const ValueKey("btn-assign-t1-alice")), ElevatedButton(style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF4338CA), foregroundColor: Colors.white, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _assignTask2ToBob(), child: Text("Assign T2 to Bob"), key: const ValueKey("btn-assign-t2-bob"))]), Row(spacing: 8, key: const ValueKey("filter-buttons"), children: [ElevatedButton(style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF4338CA), foregroundColor: Colors.white, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _filterAll(), child: Text("All"), key: const ValueKey("btn-filter-all")), OutlinedButton(style: OutlinedButton.styleFrom(foregroundColor: const Color(0xFF374151), side: const BorderSide(color: Color(0xFFE5E7EB)), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _filterHigh(), child: Text("High Priority"), key: const ValueKey("btn-filter-high"))])]), Column(spacing: 12, crossAxisAlignment: CrossAxisAlignment.start, key: const ValueKey("stats-panel"), children: [Text(style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600), "Statistics"), Text("Total tasks: 4", key: const ValueKey("stat-total")), Text('Filter: ${_filterPriority}', key: const ValueKey("stat-filter")), Text('${_errorMessage}', key: const ValueKey("stat-error")), Text('Saving: ${_isSaving}', key: const ValueKey("stat-saving")), Text('${_saveError}', key: const ValueKey("stat-save-error")), OutlinedButton(style: OutlinedButton.styleFrom(foregroundColor: const Color(0xFF374151), side: const BorderSide(color: Color(0xFFE5E7EB)), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))), onPressed: () => _syncToBackend(), child: Text("Sync to Backend"), key: const ValueKey("btn-sync"))])])]));
  }
}