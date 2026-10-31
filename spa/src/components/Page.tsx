import type { ReactNode } from "react";

import styles from "./Page.module.css";

/** The single, mobile-first, centred column every screen state lives in. */
export function Page({ children }: { readonly children: ReactNode }) {
  return <main className={styles.page}>{children}</main>;
}
