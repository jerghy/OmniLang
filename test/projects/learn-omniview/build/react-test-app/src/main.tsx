import React from 'react';
import ReactDOM from 'react-dom/client';
import { SimpleApp } from './react-output/SimpleApp';

ReactDOM.createRoot(document.getElementById('root')!).render(
  React.createElement(React.StrictMode, null, React.createElement(SimpleApp))
);
