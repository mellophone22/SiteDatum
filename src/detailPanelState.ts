export type DetailPanelKeyEvent = {
  key: string;
  defaultPrevented?: boolean;
  isComposing?: boolean;
};

export function shouldCloseDetailPanel(event: DetailPanelKeyEvent): boolean {
  return event.key === "Escape" && !event.defaultPrevented && !event.isComposing;
}
