/**
 * Wire protocol frame codec.
 *
 * Binary layout (21-byte header + variable-length data):
 *
 *   Offset  Size  Field
 *   ------  ----  -----
 *      0      1   token_type  (u8)
 *      1      4   stream_id   (u32 LE)
 *      5      4   sequence    (u32 LE)
 *      9      8   timestamp_ns (u64 LE)
 *     17      4   data_len    (u32 LE)
 *     21      N   data        (N = data_len bytes)
 *
 * Total frame size = 21 + data_len.
 */

/** Size of the fixed header preceding the variable-length data payload. */
export const HEADER_SIZE = 21;

/** A single decoded wire frame. */
export interface TokenFrame {
  tokenType: number;
  streamId: number;
  sequence: number;
  timestampNs: bigint;
  data: Uint8Array;
}

/**
 * Encode a TokenFrame into a binary ArrayBuffer suitable for WebSocket send.
 */
export function encodeFrame(frame: TokenFrame): ArrayBuffer {
  const totalSize = HEADER_SIZE + frame.data.length;
  const buffer = new ArrayBuffer(totalSize);
  const view = new DataView(buffer);
  const bytes = new Uint8Array(buffer);

  view.setUint8(0, frame.tokenType);
  view.setUint32(1, frame.streamId, true);
  view.setUint32(5, frame.sequence, true);
  view.setBigUint64(9, frame.timestampNs, true);
  view.setUint32(17, frame.data.length, true);

  bytes.set(frame.data, HEADER_SIZE);

  return buffer;
}

/**
 * Decode a single TokenFrame starting at `offset` within `buffer`.
 *
 * Returns null if the buffer does not contain enough bytes for a complete
 * frame (header + declared data_len).
 */
export function decodeFrame(buffer: ArrayBuffer, offset = 0): TokenFrame | null {
  const remaining = buffer.byteLength - offset;
  if (remaining < HEADER_SIZE) {
    return null;
  }

  const view = new DataView(buffer, offset);
  const dataLen = view.getUint32(17, true);

  if (remaining < HEADER_SIZE + dataLen) {
    return null;
  }

  return {
    tokenType: view.getUint8(0),
    streamId: view.getUint32(1, true),
    sequence: view.getUint32(5, true),
    timestampNs: view.getBigUint64(9, true),
    data: new Uint8Array(buffer, offset + HEADER_SIZE, dataLen),
  };
}

/**
 * Decode all complete frames packed contiguously in `buffer`.
 *
 * Stops when there are not enough remaining bytes for another complete frame.
 */
export function decodeFrames(buffer: ArrayBuffer): TokenFrame[] {
  const frames: TokenFrame[] = [];
  let offset = 0;

  while (offset < buffer.byteLength) {
    const frame = decodeFrame(buffer, offset);
    if (!frame) break;
    frames.push(frame);
    offset += HEADER_SIZE + frame.data.length;
  }

  return frames;
}
