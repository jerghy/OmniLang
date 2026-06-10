import React from 'react'
import ReactDOM from 'react-dom/client'
import { SignupForm } from './SignupForm'
ReactDOM.createRoot(document.getElementById('root')).render(
  React.createElement(React.StrictMode, null, React.createElement(SignupForm))
)
