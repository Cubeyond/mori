// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

/** Keep optional background bytes in this origin's browser storage, never in S3 or localStorage. */
export async function backgroundStorage(
  value?: Blob | null,
): Promise<Blob | null> {
  return new Promise((resolve, reject) => {
    const opening = indexedDB.open('mori-appearance', 1);
    let blocked = false;
    opening.onupgradeneeded = () => {
      opening.result.createObjectStore('background');
    };
    opening.onerror = () => {
      reject(new Error('Background storage is unavailable in this browser.'));
    };
    opening.onblocked = () => {
      blocked = true;
      reject(new Error('Close another Mori tab and try again.'));
    };
    opening.onsuccess = () => {
      const db = opening.result;
      if (blocked) {
        db.close();
        return;
      }
      const transaction = db.transaction(
        'background',
        value === undefined ? 'readonly' : 'readwrite',
      );
      const store = transaction.objectStore('background');
      let result: Blob | null = null;
      if (value === undefined) {
        const request = store.get('image');
        request.onsuccess = () => {
          const stored: unknown = request.result;
          if (stored instanceof Blob) result = stored;
        };
      } else if (value === null) store.delete('image');
      else store.put(value, 'image');
      transaction.oncomplete = () => {
        db.close();
        resolve(result);
      };
      transaction.onabort = () => {
        db.close();
        reject(
          new Error(
            'Could not save this background. The previous image was kept.',
          ),
        );
      };
      transaction.onerror = () => {
        /* The abort handler reports the failed transaction once. */
      };
    };
  });
}
