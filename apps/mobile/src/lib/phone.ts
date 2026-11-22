/**
 * Normalise what people type into international format for the API.
 * Pakistan is the default country: "0300 1234567" -> "+923001234567".
 * The API validates the result; this only fixes the common local forms.
 */
export function toE164(input: string, defaultCountryCode = '92'): string {
  const digits = input.replace(/[^\d+]/g, '');
  if (digits.startsWith('+')) return `+${digits.slice(1).replace(/\+/g, '')}`;
  if (digits.startsWith('00')) return `+${digits.slice(2)}`;
  if (digits.startsWith('0')) return `+${defaultCountryCode}${digits.slice(1)}`;
  return `+${digits}`;
}
