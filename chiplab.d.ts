/** The chiplab test-authoring API.
 *
 * A test drives a live simulation through the global {@link simulation}.
 * The simulation is paused whenever your test code runs and advances
 * only inside an awaited `runFor*` / `runUntil*` call, so reads between advances
 * always reflect a frozen state.
 *
 * See `/mcp/resources/docs/testing.md` for how to write and run a suite.
 */
declare namespace chiplab {
  /** Why a `runUntil*` call returned. */
  type RunUntilOutcome = { kind: "matched" | "timed_out" };

  /** A live, paused-by-default simulation. */
  interface Simulation {
    /** Advance virtual time by `millis` (1 to 60000), then pause. */
    runForMillis(millis: number): Promise<void>;

    /**
     * Advance until `expected` appears on any board UART, or `maxMillis`
     * (1 to 60000) of virtual time elapses, whichever comes first.
     */
    runUntilUartLine(
      expected: string,
      maxMillis: number,
    ): Promise<RunUntilOutcome>;
  }
}

declare const simulation: chiplab.Simulation;
