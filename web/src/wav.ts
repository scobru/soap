export type WavFormat = "pcm16" | "float32";

export function encodeWav(channels: Float32Array[], sampleRate: number, format: WavFormat): Blob {
  const numChannels = channels.length;
  const frames = channels[0].length;
  const bytesPerSample = format === "pcm16" ? 2 : 4;
  const blockAlign = numChannels * bytesPerSample;
  const dataSize = frames * blockAlign;
  const buffer = new ArrayBuffer(44 + dataSize);
  const view = new DataView(buffer);

  const writeTag = (offset: number, tag: string) => {
    for (let i = 0; i < 4; i++) view.setUint8(offset + i, tag.charCodeAt(i));
  };
  writeTag(0, "RIFF");
  view.setUint32(4, 36 + dataSize, true);
  writeTag(8, "WAVE");
  writeTag(12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, format === "pcm16" ? 1 : 3, true);
  view.setUint16(22, numChannels, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * blockAlign, true);
  view.setUint16(32, blockAlign, true);
  view.setUint16(34, bytesPerSample * 8, true);
  writeTag(36, "data");
  view.setUint32(40, dataSize, true);

  let offset = 44;
  for (let i = 0; i < frames; i++) {
    for (let c = 0; c < numChannels; c++) {
      const s = channels[c][i];
      if (format === "pcm16") {
        const clamped = Math.max(-1, Math.min(1, s));
        view.setInt16(offset, clamped < 0 ? clamped * 0x8000 : clamped * 0x7fff, true);
      } else {
        view.setFloat32(offset, s, true);
      }
      offset += bytesPerSample;
    }
  }
  return new Blob([buffer], { type: "audio/wav" });
}
