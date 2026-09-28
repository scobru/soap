export class Waveform {
  private peaks: Float32Array | null = null;
  private progress = 0;

  constructor(
    private readonly canvas: HTMLCanvasElement,
    private readonly color: () => string,
    onSeek: (fraction: number) => void,
  ) {
    canvas.addEventListener("click", (e) => {
      const rect = canvas.getBoundingClientRect();
      onSeek(Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width)));
    });
    new ResizeObserver(() => this.draw()).observe(canvas);
  }

  setAudio(channels: Float32Array[] | null) {
    this.peaks = channels ? computePeaks(channels, 2000) : null;
    this.progress = 0;
    this.draw();
  }

  setProgress(fraction: number) {
    this.progress = fraction;
    this.draw();
  }

  draw() {
    const { canvas } = this;
    const dpr = window.devicePixelRatio || 1;
    const width = Math.max(1, Math.round(canvas.clientWidth * dpr));
    const height = Math.max(1, Math.round(canvas.clientHeight * dpr));
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
    }
    const ctx = canvas.getContext("2d")!;
    ctx.clearRect(0, 0, width, height);
    const styles = getComputedStyle(canvas);
    const mid = height / 2;

    ctx.fillStyle = styles.getPropertyValue("--wave-axis");
    ctx.fillRect(0, Math.floor(mid), width, Math.max(1, dpr));
    if (!this.peaks) return;

    const bins = this.peaks.length / 2;
    const played = this.progress * width;
    const base = this.color();
    const dim = styles.getPropertyValue("--wave-dim");
    for (let x = 0; x < width; x++) {
      const bin = Math.floor((x / width) * bins);
      const min = this.peaks[bin * 2];
      const max = this.peaks[bin * 2 + 1];
      ctx.fillStyle = x <= played ? base : dim;
      const top = mid - max * mid * 0.95;
      const bottom = mid - min * mid * 0.95;
      ctx.fillRect(x, top, 1, Math.max(dpr, bottom - top));
    }
    if (this.progress > 0) {
      ctx.fillStyle = styles.getPropertyValue("--fg");
      ctx.fillRect(Math.round(played), 0, Math.max(1, dpr), height);
    }
  }
}

function computePeaks(channels: Float32Array[], bins: number): Float32Array {
  const length = channels[0].length;
  const out = new Float32Array(bins * 2);
  const size = Math.max(1, Math.floor(length / bins));
  for (let b = 0; b < bins; b++) {
    let min = 0;
    let max = 0;
    const start = b * size;
    const end = Math.min(length, start + size);
    for (const ch of channels) {
      for (let i = start; i < end; i++) {
        const v = ch[i];
        if (v < min) min = v;
        if (v > max) max = v;
      }
    }
    out[b * 2] = min;
    out[b * 2 + 1] = max;
  }
  return out;
}
