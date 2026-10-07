Deno.test("passes", async () => {
  await simulation.runForMillis(1);
});

Deno.test({
  name: "is skipped",
  ignore: true,
  fn: async () => {
    await simulation.runForMillis(1);
  },
});
