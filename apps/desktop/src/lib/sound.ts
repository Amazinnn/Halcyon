// v1.4 Web Audio chime: a short two-tone ping generated at runtime (no assets).
let ctx: AudioContext | null = null;

export function playChime() {
  try {
    ctx ??= new AudioContext();
    const now = ctx.currentTime;
    const notes: Array<[number, number, number]> = [
      [660, 0, 0.16],
      [880, 0.16, 0.3],
    ];
    for (const [freq, at, dur] of notes) {
      const o = ctx.createOscillator();
      const g = ctx.createGain();
      o.type = "sine";
      o.frequency.value = freq;
      g.gain.setValueAtTime(0.1, now + at);
      g.gain.exponentialRampToValueAtTime(0.001, now + at + dur);
      o.connect(g).connect(ctx.destination);
      o.start(now + at);
      o.stop(now + at + dur);
    }
  } catch {
    /* audio unavailable; ignore */
  }
}
