import type { DraftImage } from "./draft";

/** The image as PNG, the image type clipboards accept. A JPEG goes through a canvas. */
export async function pngBlob(image: DraftImage): Promise<Blob> {
  const blob = new Blob([image.bytes], { type: image.mime });
  if (image.mime === "image/png") return blob;
  const bitmap = await createImageBitmap(blob);
  try {
    const canvas = document.createElement("canvas");
    canvas.width = bitmap.width;
    canvas.height = bitmap.height;
    const context = canvas.getContext("2d");
    if (!context) throw new Error("no canvas context");
    context.drawImage(bitmap, 0, 0);
    return await new Promise<Blob>((resolve, reject) =>
      canvas.toBlob((png) => (png ? resolve(png) : reject(new Error("no PNG"))), "image/png"),
    );
  } finally {
    bitmap.close();
  }
}

/**
 * Writes content that may still be on its way. Call this directly in the click handler, before
 * any `await`: WebKit refuses clipboard writes once the click's task has ended. If `content`
 * rejects, the clipboard stays as it was. Resolves to whether the write happened.
 */
export function writeLater(type: string, content: Promise<Blob>): Promise<boolean> {
  content.catch(() => {});
  try {
    return navigator.clipboard.write([new ClipboardItem({ [type]: content })]).then(
      () => true,
      () => false,
    );
  } catch {
    return Promise.resolve(false);
  }
}

/** A best effort after the click has ended; WebKit may refuse it. */
export async function writeTextNow(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}
