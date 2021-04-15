export function randInt(a: number, b?: number): number {
  if (b === undefined) {
    b = a;
    a = 0;
  }
  if (b < a) {
    [a, b] = [b, a];
  }
  return Math.floor(Math.random() * (b - a) + a);
}
