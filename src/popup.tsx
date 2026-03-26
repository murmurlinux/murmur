import { render } from "solid-js/web";
import { RecordingPopup } from "./components/RecordingPopup";
import "./popup.css";

const root = document.getElementById("root");
render(() => <RecordingPopup />, root!);
