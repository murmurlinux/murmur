/* @refresh reload */
import { render } from "solid-js/web";
import { OnboardingWizard } from "./components/OnboardingWizard";

const root = document.getElementById("root");
render(() => <OnboardingWizard />, root!);
