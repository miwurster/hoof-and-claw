import mark from "../../../assets/brand/mark.svg";
import styles from "./Masthead.module.css";
import { Score } from "./Score";

/** The logo: the mark beside the wordmark, with the ampersand in Horn, and the Player's Score on the right. */
export function Masthead({ score }: { readonly score: number }) {
  return (
    <header className={styles.masthead}>
      <h1 className={styles.wordmark}>
        <img className={styles.mark} src={mark} alt="" />
        <span>
          Hoof <span className={styles.ampersand}>&amp;</span> Claw
        </span>
      </h1>
      <Score score={score} />
    </header>
  );
}
