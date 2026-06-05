import React, { useCallback, useState } from "react";
import styles from "./ShopApp.module.css";
import { useCartStore } from "./useCartStore";

export function ShopApp() {
    // .omni:10 -- component ShopApp

    const cart_count = useCartStore(s => s.cart_count());
    const cart_total = useCartStore(s => s.cart_total());
    const [search, setSearch] = useState<string>("");
    const handle_add = useCallback(() => {
        useCartStore.getState().add_item();
    }, []);
    const handle_clear = useCallback(() => {
        useCartStore.getState().clear_cart();
    }, []);

    return (
        <div>
        <div style={{flexDirection: 'row', gap: '16px', alignItems: 'center', display: 'flex'}}>
        <h2>Shop</h2>
        <div style={{flex: 1}}></div>
        <input placeholder="Search..." value={search} onChange={e => setSearch(e.target.value)} data-testid="search-input" />
        <span data-testid="cart-count">
        Items: 
        {cart_count}
        </span>
        </div>
        <div style={{display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(250px, 1fr))', gap: '16px'}} data-testid="product-grid">
        <div data-testid="product-1">
        <h2>Widget</h2>
        <span>$19.99</span>
        <button onClick={handle_add} data-testid="btn-add-1">Add to Cart</button>
        </div>
        <div data-testid="product-2">
        <h2>Gadget</h2>
        <span>$29.99</span>
        <button onClick={handle_add} data-testid="btn-add-2">Add to Cart</button>
        </div>
        <div data-testid="product-3">
        <h2>Doohickey</h2>
        <span>$9.99</span>
        <button onClick={handle_add} data-testid="btn-add-3">Add to Cart</button>
        </div>
        </div>
        <div style={{flexDirection: 'row', gap: '16px', display: 'flex'}} data-testid="cart-footer">
        <span>
        Total: 
        {cart_total}
        </span>
        <button onClick={handle_clear} data-testid="btn-clear-cart">Clear Cart</button>
        </div>
        </div>
    );
}