Deno.test("passes", async () => {
  await simulation.runForMillis(1);
});

Deno.test({
  name: "exits the test runner",
  // Deno otherwise turns the exit into a failure of only this test.
  sanitizeExit: false,
  fn: async () => {
    await simulation.runForMillis(1);
    Deno.exit(1);
  },
});

Deno.test("never runs", async () => {
  await simulation.runForMillis(1);
});
