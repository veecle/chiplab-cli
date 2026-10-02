Deno.test("always fails", async () => {
  await simulation.runForMillis(1);
  throw new Error("this test always fails");
});
