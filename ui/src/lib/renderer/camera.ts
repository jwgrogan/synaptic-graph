import type { ZoomLevel } from "../types";

export class Camera {
  x = 0;
  y = 0;
  zoom = 1;

  private minZoom = 0.05;
  private maxZoom = 8;

  pan(dx: number, dy: number) {
    this.x += dx;
    this.y += dy;
  }

  zoomAt(screenX: number, screenY: number, factor: number) {
    const newZoom = Math.max(this.minZoom, Math.min(this.maxZoom, this.zoom * factor));
    const ratio = newZoom / this.zoom;

    this.x = screenX - (screenX - this.x) * ratio;
    this.y = screenY - (screenY - this.y) * ratio;
    this.zoom = newZoom;
  }

  getZoomLevel(): ZoomLevel {
    if (this.zoom < 0.4) return "galaxy";
    if (this.zoom < 2.0) return "cluster";
    return "node";
  }

  zoomToFit(
    cx: number,
    cy: number,
    radius: number,
    screenWidth: number,
    screenHeight: number
  ) {
    const targetZoom = Math.min(screenWidth, screenHeight) / (radius * 3);
    this.zoom = Math.max(this.minZoom, Math.min(this.maxZoom, targetZoom));
    this.x = screenWidth / 2 - cx * this.zoom;
    this.y = screenHeight / 2 - cy * this.zoom;
  }
}
