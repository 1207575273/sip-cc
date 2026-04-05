import { callSnap } from "../shared/invoke";

export async function takeSnap(
  x: number, y: number, width: number, height: number
): Promise<string> {
  return callSnap({ x, y, width, height });
}
