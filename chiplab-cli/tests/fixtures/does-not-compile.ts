const value: number = "not a number";

Deno.test("never runs", () => {
  console.log(value);
});
