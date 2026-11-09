import { describe, expect, it } from 'vitest'
import { ROW, StickyCtrl, type InputEvent, type RowKey } from './keys'
import { EMPTY, type Reader } from './paste'
import { press } from './row'

const keyOf = (label: string) => ROW.find((key) => key.label === label) as RowKey

const pressing = (clipboard: Reader | undefined = { readText: async () => 'pwd' }, disabled = false) => {
  const sent: InputEvent[][] = []
  const failed: unknown[] = []
  const ctrl = new StickyCtrl()
  const send = (events: InputEvent[]) => sent.push(events)
  return { sent, failed, ctrl, with: { ctrl, disabled, send, failed: (error: unknown) => failed.push(error), clipboard } }
}

describe('press', () => {
  it('sends the event of a key', async () => {
    const row = pressing()
    await press(keyOf('Esc'), row.with)
    expect(row.sent).toEqual([[{ key: { code: 'esc' } }]])
  })

  it('arms Ctrl for the next key and sends nothing', async () => {
    const row = pressing()
    await press(keyOf('Ctrl'), row.with)
    await press(keyOf('↑'), row.with)
    expect([row.sent, row.ctrl.armed]).toEqual([[[{ key: { code: 'up', mods: ['ctrl'] } }]], false])
  })

  it('pastes the clipboard', async () => {
    const row = pressing()
    await press(keyOf('Paste'), row.with)
    expect(row.sent).toEqual([[{ paste: 'pwd' }]])
  })

  it('sends nothing and says why for an empty clipboard', async () => {
    const row = pressing({ readText: async () => '' })
    await press(keyOf('Paste'), row.with)
    expect([row.sent, row.failed]).toEqual([[], [new Error(EMPTY)]])
  })

  it('keeps Ctrl armed when the clipboard gives nothing', async () => {
    const row = pressing({ readText: async () => '' })
    await press(keyOf('Ctrl'), row.with)
    await press(keyOf('Paste'), row.with)
    expect(row.ctrl.armed).toBe(true)
  })

  it.each(ROW.map((key) => [key.label, key]))('sends nothing for %s on a closed tab', async (_, key) => {
    const row = pressing(undefined, true)
    await press(key, row.with)
    expect([row.sent, row.failed, row.ctrl.armed]).toEqual([[], [], false])
  })
})
