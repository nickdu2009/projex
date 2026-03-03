import { Button, Checkbox, Group, Loader, Modal, Stack, Text, TextInput } from '@mantine/core';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { exportApi } from '../api/export';
import { syncApi, type PendingWipeInfo } from '../api/sync';
import { showError, showSuccess } from '../utils/errorToast';
import { logger } from '../utils/logger';
import { syncManager } from '../sync/SyncManager';
import { useIsMobile } from '../utils/useIsMobile';

type Step = 'backup' | 'phrase';

export function SyncWipeGate({ enabled }: { enabled: boolean }) {
  const { t } = useTranslation();
  const isMobile = useIsMobile();
  const [pending, setPending] = useState<PendingWipeInfo | null>(null);
  const [opened, setOpened] = useState(false);
  const [checkingAfterEnable, setCheckingAfterEnable] = useState(false);
  const [step, setStep] = useState<Step>('backup');
  const [backedUp, setBackedUp] = useState(false);
  const [phrase, setPhrase] = useState('');
  const [exporting, setExporting] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [rejecting, setRejecting] = useState(false);

  const confirmPhrase = 'CLEAR';
  const phraseOk = useMemo(() => phrase.trim() === confirmPhrase, [phrase]);

  const resetUi = () => {
    setStep('backup');
    setBackedUp(false);
    setPhrase('');
  };

  const refreshPending = useCallback(async () => {
    if (!enabled) return;
    try {
      const p = await syncApi.getPendingWipe();
      setPending(p);
      // If we're in the "checking after enable" phase, keep the modal open
      // even when there's no pending wipe yet. Otherwise the modal will flash
      // and disappear while the sync is still running.
      setOpened(Boolean(p) || checkingAfterEnable);
      if (p) {
        resetUi();
      }
    } catch (e: unknown) {
      logger.debug('Get pending wipe skipped:', e);
    }
  }, [enabled, checkingAfterEnable]);

  useEffect(() => {
    refreshPending();
  }, [refreshPending]);

  // Track previous enabled value to detect the false→true transition.
  const prevEnabledRef = useRef(false);

  useEffect(() => {
    if (!enabled) {
      prevEnabledRef.current = false;
      setCheckingAfterEnable(false);
      return;
    }

    const justEnabled = !prevEnabledRef.current;
    prevEnabledRef.current = true;

    // Subscribe first so we don't miss the error event from the sync below.
    const unsub = syncManager.subscribe((s) => {
      if (s.status === 'error') {
        setCheckingAfterEnable(false);
        refreshPending();
      }
    });

    // When sync is freshly enabled, trigger an immediate sync so that any
    // pending remote WIPE_INTENT is detected right away.  The subscriber
    // above is already registered at this point, so the error event will
    // be caught and refreshPending() will fire correctly.
    if (justEnabled) {
      setCheckingAfterEnable(true);
      setOpened(true);
      // Fire-and-forget with a finally path that clears the checking UI.
      // Note: if a WIPE_INTENT exists, sync will error and subscriber will refresh pending.
      syncManager
        .sync()
        .catch(() => {
          // SYNC_WIPE_CONFIRM_REQUIRED is handled via refreshPending().
        })
        .finally(() => {
          setCheckingAfterEnable(false);
          // Re-check pending wipe once sync finishes; refreshPending will decide
          // whether to keep the modal open or close it.
          refreshPending();
        });
    }

    return unsub;
  }, [enabled, refreshPending]);

  const handleExportBackup = async () => {
    setExporting(true);
    try {
      const jsonString = await exportApi.exportJson();
      const filePath = await save({
        title: t('settings.dangerZone.backupDialogTitle'),
        filters: [{ name: 'JSON', extensions: ['json'] }],
        defaultPath: `projex-backup-${new Date().toISOString().split('T')[0]}.json`,
      });
      if (filePath) {
        await writeTextFile(filePath, jsonString);
        showSuccess(t('settings.dangerZone.backupSuccess'));
      }
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('settings.dangerZone.backupFailed'));
    } finally {
      setExporting(false);
    }
  };

  const handleConfirm = async () => {
    if (!pending) return;
    setConfirming(true);
    try {
      await syncApi.confirmWipe(pending.wipeId, phrase.trim());
      showSuccess(t('settings.dangerZone.remoteWipeConfirmed'));
      setOpened(false);
      setPending(null);
      resetUi();
      await syncManager.initialize();
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('settings.dangerZone.remoteWipeConfirmFailed'));
    } finally {
      setConfirming(false);
    }
  };

  const handleReject = async () => {
    if (!pending) return;
    setRejecting(true);
    try {
      await syncApi.rejectWipe(pending.wipeId);
      // Keep the gate modal visible until sync is actually disabled.
      // Otherwise, users can immediately re-enable sync while the disable call is still in-flight,
      // and the gate may miss the enable transition (race) and not trigger the follow-up sync.
      await syncManager.setEnabled(false);
      showSuccess(t('settings.dangerZone.remoteWipeRejected'));
      setOpened(false);
      setPending(null);
      resetUi();
    } catch (e: unknown) {
      showError((e as { message?: string })?.message ?? t('settings.dangerZone.remoteWipeRejectFailed'));
    } finally {
      setRejecting(false);
    }
  };

  if (!enabled) return null;
  if (!pending && !checkingAfterEnable) return null;

  return (
    <Modal
      opened={opened}
      onClose={() => {}}
      title={t('settings.dangerZone.remoteWipeTitle')}
      centered
      size={isMobile ? '100%' : 'sm'}
      fullScreen={isMobile}
      closeOnClickOutside={false}
      closeOnEscape={false}
      withCloseButton={false}
    >
      <Stack>
        {checkingAfterEnable && !pending && (
          <>
            <Text size="sm">
              {t('settings.dangerZone.syncCheckingRemoteChanges')}
            </Text>
            <Text size="sm" c="dimmed">
              {t('settings.dangerZone.syncCheckingRemoteChangesHint')}
            </Text>
            <Group justify="center">
              <Loader size="sm" />
            </Group>
          </>
        )}

        {pending && (
        <Text size="sm">
          {t('settings.dangerZone.remoteWipeMessage', {
            deviceId: pending.sourceDeviceId,
            createdAt: pending.createdAt,
          })}
        </Text>
        )}

        {pending && step === 'backup' && (
          <>
            <Text size="sm" c="dimmed">
              {t('settings.dangerZone.backupReminder')}
            </Text>
            <Group justify={isMobile ? 'stretch' : 'space-between'} wrap="wrap" gap="xs">
              <Button variant="light" onClick={handleExportBackup} loading={exporting} fullWidth={isMobile}>
                {t('settings.dangerZone.backupNow')}
              </Button>
              <Checkbox
                checked={backedUp}
                onChange={(e) => setBackedUp(e.currentTarget.checked)}
                label={t('settings.dangerZone.backupAck')}
              />
            </Group>
            <Group justify={isMobile ? 'stretch' : 'flex-end'} wrap="wrap">
              <Button variant="light" color="red" onClick={handleReject} loading={rejecting} fullWidth={isMobile}>
                {t('settings.dangerZone.rejectAndDisconnect')}
              </Button>
              <Button onClick={() => setStep('phrase')} disabled={!backedUp} fullWidth={isMobile}>
                {t('common.continue')}
              </Button>
            </Group>
          </>
        )}

        {pending && step === 'phrase' && (
          <>
            <Text size="sm" c="dimmed">
              {t('settings.dangerZone.phraseHint', { phrase: confirmPhrase })}
            </Text>
            <TextInput
              label={t('settings.dangerZone.phraseLabel')}
              value={phrase}
              onChange={(e) => setPhrase(e.currentTarget.value)}
              placeholder={confirmPhrase}
            />
            <Group justify={isMobile ? 'stretch' : 'flex-end'} wrap="wrap">
              <Button variant="subtle" onClick={() => setStep('backup')} disabled={confirming || rejecting} fullWidth={isMobile}>
                {t('common.back')}
              </Button>
              <Button
                color="red"
                onClick={handleConfirm}
                loading={confirming}
                disabled={!phraseOk}
                fullWidth={isMobile}
              >
                {t('common.confirm')}
              </Button>
            </Group>
          </>
        )}
      </Stack>
    </Modal>
  );
}

