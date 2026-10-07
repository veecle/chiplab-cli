Deno.test("firmware greets", async () => {
  const outcome = await simulation.runUntilUartLine("Hello world!", 10000);
  if (outcome.kind !== "matched") throw new Error(outcome.kind);
});
