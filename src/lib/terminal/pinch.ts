export type Point = [number, number]

const apart = ([[ax, ay], [bx, by]]: Point[]) => Math.hypot(ax - bx, ay - by)

export class Pinch {
  #points = new Map<number, Point>()
  #spread = 0

  press(id: number, point: Point) {
    this.#points.set(id, point)
    this.#spread = this.#points.size === 2 ? apart([...this.#points.values()]) : 0
    return this.#spread > 0
  }

  move(id: number, point: Point) {
    if (!this.#points.has(id)) return null
    this.#points.set(id, point)
    return this.#spread > 0 ? apart([...this.#points.values()]) / this.#spread : null
  }

  lift(id: number) {
    this.#points.delete(id)
    this.#spread = 0
  }
}
