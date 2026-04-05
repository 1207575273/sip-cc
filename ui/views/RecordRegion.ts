/**
 * 录制区域指示框——透明窗口，只显示一个红色边框标识录制范围。
 * 窗口设置为 ignore_cursor_events，鼠标可以穿透。
 * URL 参数传入区域坐标：?view=record-region&x=100&y=200&w=800&h=600
 */
export function mountRecordRegion(container: HTMLElement): void {
  const params = new URLSearchParams(window.location.search);
  const x = parseInt(params.get("x") || "0");
  const y = parseInt(params.get("y") || "0");
  const w = parseInt(params.get("w") || "400");
  const h = parseInt(params.get("h") || "300");

  const wrapper = document.createElement("div");
  wrapper.id = "record-region-border";
  wrapper.innerHTML = `
    <div class="region-border" style="left:0;top:0;width:100%;height:100%"></div>
    <div class="region-label">REC ${w}×${h}</div>
  `;

  // 标签定位到左上角
  const label = wrapper.querySelector(".region-label") as HTMLElement;
  label.style.top = "-22px";
  label.style.left = "0";

  container.appendChild(wrapper);
}
