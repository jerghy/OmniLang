const O = window.__OMNI__;

// Switch to design mode so we can create elements
O.switchMode('design');

// Root Card container with padding
const card = O.card({ padding: '24px' });

// 1. Page header
O.heading('My Tasks', '1', card.id);

// 2. Input field + Add button in a horizontal flex row
const inputRow = O.flex('row', '8px', {}, card.id);
O.input('New task...', {}, inputRow.id);
O.button('Add', { variant: 'primary' }, inputRow.id);

// 3. List area (initially empty — tasks will appear here)
const listArea = O.container({}, card.id);

// 4. Footer showing remaining task count
O.text('0 tasks remaining', {}, card.id);

// 5. Signal code block to track the task count
O.codeBlock('signal', 'taskCount', {
  typeAnnotation: 'i32',
  source: `// Signal: taskCount
// Tracks the number of remaining tasks
let taskCount = 0;`
});
