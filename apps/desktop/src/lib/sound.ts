// v1.4 Web Audio chime: a short two-tone ping generated at runtime (no assets).
// v1.11.1: schedule both oscillators on a fresh 10ms offset so they never
// collide at the same timestamp (earlier this caused overlap + echo), and
// lower the gain to avoid clipping.
let ctx: AudioContext | null = null;

export function playChime() {
  try {
    ctx ??= new AudioContext();
    const now = ctx.currentTime + 0.01;
    const notes: Array<[number, number, number]> = [
      [660, 0, 0.14],
      [880, 0.14, 0.26],
    ];
    for (const [freq, at, dur] of notes) {
      const o = ctx.createOscillator();
      const g = ctx.createGain();
      o.type = "sine";
      o.frequency.value = freq;
      g.gain.setValueAtTime(0.08, now + at);
      g.gain.exponentialRampToValueAtTime(0.001, now + at + dur);
      o.connect(g).connect(ctx.destination);
      o.start(now + at);
      o.stop(now + at + dur);
    }
  } catch {
    /* audio unavailable; ignore */
  }
}
