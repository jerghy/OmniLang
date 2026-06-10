import React, { useState } from "react";
import styles from "./StyleShowcase.module.css";

export function StyleShowcase() {
    // .omni:5 -- component StyleShowcase

    const [theme, setTheme] = useState<string>("light");
    const [selected_card, setSelected_card] = useState<number>(1);

    return (
        <div data-testid="showcase">
        <h1 className={styles.heading} data-testid="page-title">OmniView CSS Showcase</h1>
        <div className={styles.card_typo} data-testid="card-typo">
        <h2 className={styles.heading}>Typography</h2>
        <span data-testid="text-normal">Normal text (16px)</span>
        <span className={styles.text_large} data-testid="text-large">Large bold heading</span>
        <span className={styles.text_muted} data-testid="text-muted">Muted secondary text</span>
        </div>
        <div className={styles.card_colors} data-testid="card-colors">
        <h2 className={styles.heading}>Colors</h2>
        <div className={styles.color_swatch_red} data-testid="color-swatch-red"></div>
        <div className={styles.color_swatch_blue} data-testid="color-swatch-blue"></div>
        <div className={styles.color_swatch_green} data-testid="color-swatch-green"></div>
        </div>
        <div className={styles.card_borders} data-testid="card-borders">
        <h2 className={styles.heading}>Borders</h2>
        <div className={styles.box_rounded} data-testid="box-rounded">Rounded</div>
        <div className={styles.box_bordered} data-testid="box-bordered">Bordered</div>
        <div className={styles.box_dashed} data-testid="box-dashed">Dashed</div>
        <div className={styles.box_pill} data-testid="box-pill">Pill shaped</div>
        </div>
        <div className={styles.card_shadows} data-testid="card-shadows">
        <h2 className={styles.heading}>Shadows</h2>
        <div className={styles.box_shadow_sm} data-testid="box-shadow-sm">SM</div>
        <div className={styles.box_shadow_md} data-testid="box-shadow-md">MD</div>
        <div className={styles.box_shadow_lg} data-testid="box-shadow-lg">LG</div>
        <div className={styles.box_shadow_xl} data-testid="box-shadow-xl">XL</div>
        </div>
        <div className={styles.card_layout} data-testid="card-layout">
        <h2 className={styles.heading}>Layout</h2>
        <div style={{flexDirection: 'row', gap: '16px', display: 'flex'}} data-testid="flex-row">
        <div className={styles.flex_child_1} data-testid="flex-child-1">A</div>
        <div className={styles.flex_child_2} data-testid="flex-child-2">B</div>
        <div className={styles.flex_child_3} data-testid="flex-child-3">C</div>
        </div>
        <div style={{flexDirection: 'column', gap: '8px', display: 'flex'}} data-testid="flex-col">
        <div className={styles.flex_stack_1} data-testid="flex-stack-1">Top</div>
        <div className={styles.flex_stack_2} data-testid="flex-stack-2">Bottom</div>
        </div>
        </div>
        <div className={styles.card_spacing} data-testid="card-spacing">
        <h2 className={styles.heading}>Spacing</h2>
        <div className={styles.box_padded} data-testid="box-padded">Padded content</div>
        <div className={styles.box_margined} data-testid="box-margined">Margined content</div>
        </div>
        <span className={styles.footer_text} data-testid="footer-text">
        OmniView CSS Test — 
        {theme}
        theme
        </span>
        </div>
    );
}