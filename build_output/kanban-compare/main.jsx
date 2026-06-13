import React from 'react'
import ReactDOM from 'react-dom/client'
import { KanbanBoard } from './KanbanBoard'

ReactDOM.createRoot(document.getElementById('root')).render(
  React.createElement(React.StrictMode, null, React.createElement(KanbanBoard))
)