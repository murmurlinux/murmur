/* @refresh reload */
import { render } from "solid-js/web";
import "./styles.css";

// Tray-only app: main window stays hidden.
// Settings and recording popup are separate windows.
const root = document.getElementById("root");
render(() => <div />, root!);
