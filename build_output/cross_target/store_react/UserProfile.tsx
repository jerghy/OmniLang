import React, { useCallback, useState } from "react";
import { useUserStore } from "./useUserStore";

export function UserProfile() {
    // .omni:18 -- component UserProfile

    const display_name = useUserStore(s => s.display_name());
    const login_count = useUserStore(s => s.login_count);
    const [input_name, setInput_name] = useState<string>("");
    const handle_login = useCallback(() => {
        useUserStore.getState().login(input_name);
        setInput_name("");
    }, []);
    const handle_logout = useCallback(() => {
        useUserStore.getState().logout();
    }, []);

    return (
        <div>
        <h2 data-testid="welcome">
        Welcome, 
        {display_name}
        !
        </h2>
        <span data-testid="login-count">
        Logins: 
        {login_count}
        </span>
        <div style={{flexDirection: 'row', gap: '8px', display: 'flex'}}>
        <input placeholder="Enter name" value={input_name} onChange={e => setInput_name(e.target.value)} data-testid="input-name" />
        <button onClick={handle_login} data-testid="btn-login">Login</button>
        <button onClick={handle_logout} data-testid="btn-logout">Logout</button>
        </div>
        </div>
    );
}