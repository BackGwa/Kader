import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";

document.documentElement.classList.toggle("mac", navigator.platform.startsWith("Mac"));

mount(App, { target: document.getElementById("app")! });
