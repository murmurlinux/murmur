/* @refresh reload */
import { render } from "solid-js/web";
import { SettingsPanel } from "./components/SettingsPanel";

const root = document.getElementById("root");
render(() => <SettingsPanel />, root!);
