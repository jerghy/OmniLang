import React from 'react';
import ReactDOM from 'react-dom/client';
import { TaskBoard } from './TaskBoard';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <TaskBoard />
  </React.StrictMode>,
);
