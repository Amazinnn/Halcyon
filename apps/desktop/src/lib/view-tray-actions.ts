export class ViewTrayActions {
  open = false;
  busy = false;

  constructor(private readonly restoreWindow: (label: string) => Promise<void>) {}

  toggle(): boolean {
    if (this.busy) return this.open;
    this.open = !this.open;
    return this.open;
  }

  close() {
    if (!this.busy) this.open = false;
  }

  async restore(label: string): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    this.open = false;
    try {
      await this.restoreWindow(label);
    } finally {
      this.busy = false;
    }
  }
}
