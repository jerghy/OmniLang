import React, { useCallback, useState } from "react";

export function ShoppingCart() {
    // .omni:1 -- component ShoppingCart

    const [cartCount, setCartCount] = useState<number>(0);
    const [cartTotal, setCartTotal] = useState<number>(0);
    const [lastAdded, setLastAdded] = useState<string>("");
    const addApple = useCallback(() => {
        setCartCount(prev => (prev + 1));
        setCartTotal(prev => (prev + 5));
        setLastAdded("Apple ($5)");
    }, []);
    const addBanana = useCallback(() => {
        setCartCount(prev => (prev + 1));
        setCartTotal(prev => (prev + 3));
        setLastAdded("Banana ($3)");
    }, []);
    const addOrange = useCallback(() => {
        setCartCount(prev => (prev + 1));
        setCartTotal(prev => (prev + 4));
        setLastAdded("Orange ($4)");
    }, []);
    const clearCart = useCallback(() => {
        setCartCount(0);
        setCartTotal(0);
        setLastAdded("");
    }, []);

    return (
        <div style={{padding: '24px', backgroundColor: '#FFFFFF', borderRadius: '8px', boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}}>
        <h2 data-testid="cart-title">Shopping Cart</h2>
        <h3>Products</h3>
        <div style={{flexDirection: 'row', gap: '12px', display: 'flex'}}>
        <span>Apple - $5</span>
        <button style={{backgroundColor: '#5252E0', color: '#FFFFFF', border: 'none', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={addApple} data-testid="btn-add-apple">Add to Cart</button>
        </div>
        <div style={{flexDirection: 'row', gap: '12px', display: 'flex'}}>
        <span>Banana - $3</span>
        <button style={{backgroundColor: '#5252E0', color: '#FFFFFF', border: 'none', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={addBanana} data-testid="btn-add-banana">Add to Cart</button>
        </div>
        <div style={{flexDirection: 'row', gap: '12px', display: 'flex'}}>
        <span>Orange - $4</span>
        <button style={{backgroundColor: '#5252E0', color: '#FFFFFF', border: 'none', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={addOrange} data-testid="btn-add-orange">Add to Cart</button>
        </div>
        <h3 data-testid="cart-section">Your Cart</h3>
        <span data-testid="cart-count">
        Items: 
        {cartCount}
        </span>
        <span data-testid="cart-total">
        Total: $
        {cartTotal}
        </span>
        <span data-testid="last-added">{lastAdded}</span>
        <button style={{backgroundColor: 'transparent', color: '#1F2937', border: '1px solid #E5E7EB', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={clearCart} data-testid="btn-clear">Clear Cart</button>
        </div>
    );
}