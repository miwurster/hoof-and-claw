import mark from "../../../assets/brand/mark.svg";
import styles from "./Masthead.module.css";

/** The logo: the mark beside the wordmark, with the ampersand in Horn. */
export function Masthead() {
  return (
    <header className={styles.masthead}>
      <h1 className={styles.wordmark}>
        <img className={styles.mark} src={mark} alt="" />
        <span>
          Hoof <span className={styles.ampersand}>&amp;</span> Claw
        </span>
      </h1>
    </header>
  );
}
