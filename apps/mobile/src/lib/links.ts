/** Outbound links. The store id comes from app.json, so a rename follows. */
import Constants from 'expo-constants';

const androidPackage = Constants.expoConfig?.android?.package ?? 'app.voclara';

export const PLAY_STORE_URL = `https://play.google.com/store/apps/details?id=${androidPackage}`;

/** Text for sharing a result. Always "estimated" and "IELTS-style": never an official score. */
export function shareResultMessage(band: number, mock: boolean): string {
  const what = mock ? 'an IELTS-style speaking mock' : 'English speaking practice';
  return `I got an estimated band ${band.toFixed(1)} in ${what} on Voclara. Practise speaking with me: ${PLAY_STORE_URL}`;
}
