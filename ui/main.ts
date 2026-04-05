import { mountSnapOverlay } from "./views/SnapOverlay";
import { mountGifOverlay } from "./views/GifOverlay";
import { mountRecordBar } from "./views/RecordBar";

const container = document.getElementById("app")!;
const params = new URLSearchParams(window.location.search);
const view = params.get("view");

switch (view) {
  case "snap-overlay":
    mountSnapOverlay(container);
    break;
  case "gif-overlay":
    mountGifOverlay(container);
    break;
  case "record-bar":
    mountRecordBar(container);
    break;
  default:
    break;
}
