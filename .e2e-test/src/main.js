import { Toggle } from "./Toggle.js";
import { TodoList } from "./TodoList.js";

const root = document.getElementById("root");

// Create sections
const sec1 = document.createElement("section");
sec1.id = "toggle-section";
sec1.innerHTML = "<h2>@if Test</h2>";
root.appendChild(sec1);

const sec2 = document.createElement("section");
sec2.id = "todolist-section";
sec2.innerHTML = "<h2>@for Test</h2>";
root.appendChild(sec2);

Toggle(document.getElementById("toggle-section"));
TodoList(document.getElementById("todolist-section"));
