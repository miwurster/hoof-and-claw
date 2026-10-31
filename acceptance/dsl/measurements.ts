// What a performance case measures, the budgets it is held to, and the file that keeps the numbers (ADR 0015).
import { writeFileSync } from "node:fs";

import type { TestInfo } from "@playwright/test";
import { build } from "hdr-histogram-js";
import type { Histogram } from "hdr-histogram-js";

/** The five metrics of ADR 0015. */
export type Metric = "priceFanOut" | "placingAGuess" | "resolutionBurst" | "firstEvent" | "residentMemory";

/** The most a metric may reach: its 95th percentile, its maximum, or both. */
export type Budget = { readonly p95?: number; readonly max?: number };

/** The budgets of every metric, as the harness's configuration in playwright.config.ts. */
export type Budgets = Readonly<Record<Metric, Budget>>;

/** The unit each metric is recorded and budgeted in. */
const UNIT = {
  priceFanOut: "ms",
  placingAGuess: "ms",
  resolutionBurst: "ms",
  firstEvent: "ms",
  residentMemory: "MB",
} as const satisfies Record<Metric, string>;

/** What a metric is, in glossary terms, for a failure message. */
const NAME = {
  priceFanOut: "the time from sending a price to its arrival on the last stream",
  placingAGuess: "the time to place a Guess",
  resolutionBurst: "the time from the price that makes every Guess due to the last Guess resolved event",
  firstEvent: "the time from opening a stream to its first event",
  residentMemory: "the server's resident memory",
} as const satisfies Record<Metric, string>;

/** Values go into the histogram in thousandths of their unit, as it records only integers. */
const SCALE = 1000;

/** One metric's numbers, as the measurements file keeps them. */
export type Summary = {
  readonly unit: string;
  readonly count: number;
  readonly p50: number;
  readonly p95: number;
  readonly max: number;
  readonly budget: Budget;
};

/** A breach of a budget, in glossary terms, or none. */
function breach(metric: Metric, summary: Summary): string | undefined {
  const { budget, unit } = summary;

  if (budget.p95 !== undefined && summary.p95 >= budget.p95) {
    return `${NAME[metric]} should stay under p95 ${String(budget.p95)} ${unit}, but its p95 is ${String(summary.p95)} ${unit}`;
  }

  if (budget.max !== undefined && summary.max >= budget.max) {
    return `${NAME[metric]} should stay under ${String(budget.max)} ${unit}, but reached ${String(summary.max)} ${unit}`;
  }

  return undefined;
}

/** The samples of one performance case, kept in a histogram per metric. */
export class Measurements {
  private readonly histograms = new Map<Metric, Histogram>();
  private readonly errors: string[] = [];
  private streamsClosed = 0;

  constructor(private readonly budgets: Budgets) {}

  record(metric: Metric, value: number): void {
    let histogram = this.histograms.get(metric);

    if (histogram === undefined) {
      histogram = build();
      this.histograms.set(metric, histogram);
    }

    histogram.recordValue(Math.max(1, Math.round(value * SCALE)));
  }

  recordErrors(errors: readonly string[]): void {
    this.errors.push(...errors);
  }

  recordStreamsClosedByTheServer(count: number): void {
    this.streamsClosed = count;
  }

  /** A metric's summary, or `undefined` before it has a sample. */
  summary(metric: Metric): Summary | undefined {
    const histogram = this.histograms.get(metric);

    if (histogram === undefined) {
      return undefined;
    }

    return {
      unit: UNIT[metric],
      count: histogram.totalCount,
      p50: histogram.getValueAtPercentile(50) / SCALE,
      p95: histogram.getValueAtPercentile(95) / SCALE,
      max: histogram.maxValue / SCALE,
      budget: this.budgets[metric],
    };
  }

  /** How the metric breaches its budget, or `undefined` while it keeps to it. */
  breach(metric: Metric): string | undefined {
    const summary = this.summary(metric);

    return summary === undefined ? `${NAME[metric]} has no sample` : breach(metric, summary);
  }

  /** Every number of the case, for the measurements file. */
  report(): string {
    const metrics = Object.fromEntries(
      [...this.histograms.keys()].map((metric) => [metric, this.summary(metric)] as const),
    );

    return JSON.stringify({ metrics, errors: this.errors, streamsClosedByTheServer: this.streamsClosed }, null, 2);
  }

  /** Writes every number to measurements.json in the case's output folder, which CI keeps, and attaches it. */
  async keep(testInfo: TestInfo): Promise<void> {
    const report = this.report();

    writeFileSync(testInfo.outputPath("measurements.json"), report);
    await testInfo.attach("measurements", { body: report, contentType: "application/json" });
  }

  destroy(): void {
    for (const histogram of this.histograms.values()) {
      histogram.destroy();
    }
  }
}
