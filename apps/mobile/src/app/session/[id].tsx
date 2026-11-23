import { useLocalSearchParams } from 'expo-router';

import { Screen } from '@/components';
import { FeedbackView, useFeedback } from '@/components/FeedbackView';

export default function SessionFeedback() {
  const { id } = useLocalSearchParams<{ id: string }>();
  const { fb, error } = useFeedback(id ?? null);
  return (
    <Screen>
      <FeedbackView fb={fb} error={error} />
    </Screen>
  );
}
