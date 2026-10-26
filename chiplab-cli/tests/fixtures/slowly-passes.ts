Deno.test("passes after a while", async () => {
  // Outlasts a single wait for the job, so that the job is seen running.
  await new Promise((resolve) => setTimeout(resolve, 25_000));
});
