export type Unlisten = () => void;
export function subscribeWithCleanup(
  registration: Promise<Unlisten>,
  onError?: (error: unknown) => void,
): () => void;
