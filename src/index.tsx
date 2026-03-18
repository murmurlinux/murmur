/* @refresh reload */
import { render } from "solid-js/web";
import { GadgetWindow } from "./components/GadgetWindow";
import "./styles.css";

const root = document.getElementById("root");
render(() => <GadgetWindow />, root!);
