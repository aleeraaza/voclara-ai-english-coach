/**
 * Share a graded result: the server-made card image (PNG with the band and the
 * Play Store link) through the Android share sheet, or plain text when the
 * image cannot be fetched (offline, web, sharing unavailable).
 */
import { File, Paths } from 'expo-file-system';
import * as Sharing from 'expo-sharing';
import { Platform, Share } from 'react-native';

import { API_URL, tokens } from './api/client';
import { shareResultMessage } from './links';

export async function shareResult(sessionId: string, band: number, mock: boolean): Promise<void> {
  if (Platform.OS !== 'web' && (await Sharing.isAvailableAsync().catch(() => false))) {
    try {
      const token = await tokens.getAccessToken();
      if (token) {
        const destination = new File(Paths.cache, `voclara-result-${sessionId}.png`);
        const file = await File.downloadFileAsync(`${API_URL}/api/v1/sessions/${sessionId}/share-card.png`, destination, {
          headers: { Authorization: `Bearer ${token}` },
          idempotent: true,
        });
        await Sharing.shareAsync(file.uri, { mimeType: 'image/png', dialogTitle: 'Share your result' });
        return;
      }
    } catch {
      // Fall through to the text share below.
    }
  }
  await Share.share({ message: shareResultMessage(band, mock) });
}
