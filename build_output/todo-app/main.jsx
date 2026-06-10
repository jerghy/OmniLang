import React from 'react'
import ReactDOM from 'react-dom/client'
import { TodoApp } from './TodoApp'

// Check if store has the B1 bug before mounting
import { useTodoStore } from './useTodoStore'
const store = useTodoStore.getState()
console.log('[OmniLang Todo] Store state:', {
  tasks: store.tasks,
  completed: store.completed,
  activeCount: typeof store.active_count === 'function' ? store.active_count() : 'ERROR: not a function',
  totalCount: typeof store.total_count === 'function' ? store.total_count() : 'ERROR: not a function',
})

ReactDOM.createRoot(document.getElementById('root')).render(
  React.createElement(React.StrictMode, null,
    React.createElement(TodoApp)
  )
)
