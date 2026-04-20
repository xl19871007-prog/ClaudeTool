import { useEffect } from 'react';
import { Workbench } from '@/pages/Workbench';
import { ReadinessWizard } from '@/components/ReadinessWizard';
import { LoginPromptDialog } from '@/components/LoginPromptDialog';
import { useEnv } from '@/store/env';

export default function App() {
  const refresh = useEnv((s) => s.refresh);
  const loadConfig = useEnv((s) => s.loadConfig);

  useEffect(() => {
    void refresh();
    void loadConfig();
  }, [refresh, loadConfig]);

  return (
    <>
      <Workbench />
      <ReadinessWizard />
      <LoginPromptDialog />
    </>
  );
}
