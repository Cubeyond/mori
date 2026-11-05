/** Native WebCodecs conversion streams finalized MP4 output to OPFS with bounded execution. */
import {
  Input,
  Output,
  BlobSource,
  ALL_FORMATS,
  Mp4OutputFormat,
  StreamTarget,
  Conversion,
  Quality,
  canEncodeVideo,
} from 'mediabunny';
import type { OptimizationRequest } from './model';
import { copyDirectory, copyName } from './storage';

/** Yield between bounded encode slices, including on software-only devices. */
async function rest(milliseconds: number): Promise<void> {
  await new Promise<void>((resolve) => {
    setTimeout(resolve, milliseconds);
  });
}
/** Only known eight-bit SDR profiles are transformed; uncertain high-fidelity formats stay original. */
function standardVideoProfile(codec: string | null): boolean {
  return (
    codec !== null &&
    (/^avc[13]\.(42|4d|58|64)/iu.test(codec) ||
      /^vp09\.00\.\d{2}\.08/iu.test(codec) ||
      /^av01\.0\.\d{2}[mh]\.08/iu.test(codec) ||
      /^(?:hvc1|hev1)\.1\./iu.test(codec) ||
      codec === 'vp8')
  );
}
/** Abort unsuitable or slow work without allocating a whole-video ArrayBuffer. */
export async function optimizeVideo(
  request: OptimizationRequest,
): Promise<boolean> {
  if (
    typeof VideoEncoder === 'undefined' ||
    typeof VideoDecoder === 'undefined'
  )
    return false;
  const input = new Input({
    source: new BlobSource(request.file, { maxCacheSize: 2 << 20 }),
    formats: ALL_FORMATS,
  });
  let conversion: Conversion | undefined, output: Output | undefined;
  let successful = false;
  const started = performance.now();
  try {
    const tracks = await input.getTracks(),
      videos = await input.getVideoTracks(),
      audio = await input.getAudioTracks();
    const track = videos[0];
    if (
      videos.length !== 1 ||
      track === undefined ||
      tracks.length !== videos.length + audio.length
    )
      return false;
    const [
      width,
      height,
      hdr,
      color,
      transparent,
      codec,
      decodable,
      statistics,
      pixelAspect,
    ] = await Promise.all([
      track.getDisplayWidth(),
      track.getDisplayHeight(),
      track.hasHighDynamicRange(),
      track.getColorSpace(),
      track.canBeTransparent(),
      track.getCodecParameterString(),
      track.canDecode(),
      track.computePacketStats(60),
      track.getPixelAspectRatio(),
    ]);
    if (
      pixelAspect.num !== pixelAspect.den ||
      hdr ||
      transparent ||
      !decodable ||
      !standardVideoProfile(codec) ||
      width * height > 3840 * 2160 ||
      (color.primaries != null &&
        !['bt709', 'smpte170m', 'bt470bg'].includes(color.primaries)) ||
      (color.transfer != null &&
        !['bt709', 'smpte170m', 'bt470bg', 'iec61966-2-1'].includes(
          color.transfer,
        ))
    )
      return false;
    const format = new Mp4OutputFormat({ fastStart: 'reserve' });
    for (const sound of audio) {
      const soundCodec = await sound.getCodec();
      if (
        soundCodec === null ||
        !format.getSupportedAudioCodecs().includes(soundCodec)
      )
        return false;
    }
    const quality = new Quality(
      request.mode === 'quality' ? 'very-high' : 'high',
    );
    const config = {
      width,
      height,
      quality,
      frameRate: statistics.averagePacketRate || 30,
    };
    const hardware = await canEncodeVideo('avc', {
      ...config,
      hardwareAcceleration: 'prefer-hardware',
    });
    const acceleration = hardware ? 'prefer-hardware' : 'prefer-software';
    // Software work is restricted to modest clips on machines with spare logical cores.
    if (
      !hardware &&
      ((navigator.hardwareConcurrency || 1) < 4 ||
        width * height > 1920 * 1080 ||
        !(await canEncodeVideo('avc', {
          ...config,
          hardwareAcceleration: acceleration,
        })))
    )
      return false;
    const directory = await copyDirectory(),
      handle = await directory.getFileHandle(copyName(request.copy), {
        create: true,
      });
    const writable = await handle.createWritable();
    let largestPosition = 0;
    const stream = new WritableStream<import('mediabunny').StreamTargetChunk>({
      /** Stop growing output as soon as a useful size reduction becomes impossible. */
      async write(chunk): Promise<void> {
        largestPosition = Math.max(
          largestPosition,
          chunk.position + chunk.data.byteLength,
        );
        if (largestPosition > request.file.size * 0.95)
          throw new Error('Original is already compact');
        await writable.write(chunk);
      },
      /** Publish only complete finalized files for multipart upload. */
      async close(): Promise<void> {
        await writable.close();
      },
      /** Discard partial bytes when encoding fails or exceeds a budget. */
      async abort(): Promise<void> {
        await writable.abort();
      },
    });
    output = new Output({
      format,
      target: new StreamTarget(stream, { chunked: true, chunkSize: 1 << 20 }),
    });
    conversion = await Conversion.init({
      input,
      output,
      tracks: 'all',
      showWarnings: false,
      video: {
        codec: 'avc',
        quality,
        forceTranscode: true,
        hardwareAcceleration: acceleration,
      },
    });
    if (
      !conversion.isValid ||
      conversion.discardedTracks.length > 0 ||
      conversion.utilizedTracks.length !== tracks.length
    )
      return false;
    let fraction = 0;
    conversion.onProgress = (value): void => {
      fraction = Math.max(fraction, value);
    };
    const activeConversion = conversion;
    const finished = (): boolean => activeConversion.state === 'done';
    while (!finished()) {
      const pause = new AbortController(),
        timer = setTimeout(
          () => {
            pause.abort();
          },
          hardware ? 180 : 80,
        );
      try {
        await conversion.execute({ pauseSignal: pause.signal });
      } finally {
        clearTimeout(timer);
      }
      const elapsed = performance.now() - started;
      if (
        !finished() &&
        (elapsed > request.budgetMs ||
          (elapsed > 2500 &&
            (fraction < 0.01 || elapsed / fraction > request.budgetMs * 1.1)))
      )
        return false;
      if (!finished()) await rest(hardware ? 40 : 120);
    }
    const file = await handle.getFile();
    if (file.size >= 16 && file.size <= request.file.size * 0.95)
      successful = true;
    return successful;
  } finally {
    if (!successful) {
      try {
        if (conversion !== undefined) await conversion.cancel();
        else if (output !== undefined) await output.cancel();
      } catch {
        /* The failed stream may already be closed. */
      }
    }
    input.dispose();
  }
}
