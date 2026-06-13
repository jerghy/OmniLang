import React, { useCallback, useState } from "react";

export function Counter() {
    // .omni:3 -- component Counter

    const [count, setCount] = useState<number>(0);
    const increment = useCallback(() => {
        setCount((count + 1));
    }, []);
    const decrement = useCallback(() => {
        setCount((count - 1));
    }, []);
    const reset = useCallback(() => {
        setCount(0);
    }, []);

    return (
        <div>
        <h1 data-testid="counter-label">
        Count: 
        {count}
        </h1>
        <div style={{flexDirection: 'row', gap: '8px', display: 'flex'}}>
        <button onClick={decrement} data-testid="btn-decrement">-</button>
        <button onClick={increment} data-testid="btn-increment">+</button>
        <button onClick={reset} data-testid="btn-reset">Reset</button>
        </div>
        </div>
    );
}