import React, { useCallback, useState } from "react";
import styles from "./SignupForm.module.css";
import { useFormStore } from "./useFormStore";

export function SignupForm() {
    // .omni:21 -- component SignupForm

    const submit_count = useFormStore(s => s.submit_count);
    const submitted = useFormStore(s => s.submitted);
    const [name, setName] = useState<string>("");
    const [email, setEmail] = useState<string>("");
    const [age_val, setAge_val] = useState<string>("");
    const [plan, setPlan] = useState<string>("basic");
    const [accepted_terms, setAccepted_terms] = useState<boolean>(false);
    const [errors, setErrors] = useState<string>("");
    const handle_submit = useCallback(() => {
        if ((name === "")) {
            setErrors("Name is required");
        }
        if (((name !== "") && (email === ""))) {
            setErrors("Email is required");
        }
        if (((name !== "") && (email !== ""))) {
            setErrors("");
            useFormStore.getState().submit();
        }
    }, [email, errors, name]);
    const handle_reset = useCallback(() => {
        setName("");
        setEmail("");
        setAge_val("");
        setPlan("basic");
        setAccepted_terms(false);
        setErrors("");
        useFormStore.getState().reset();
    }, [accepted_terms, age_val, email, errors, name, plan]);
    const get_error = useCallback(() => {
        setErrors(errors);
    }, [errors]);

    return (
        <div data-testid="signup-form">
        <h1 className={styles.heading} data-testid="form-title">Sign Up</h1>
        {submitted ? <>
        <div data-testid="success-message">
        <h2 className={styles.heading}>
        Welcome, 
        {name}
        !
        </h2>
        <span data-testid="success-email">
        We sent a confirmation to 
        {email}
        </span>
        <button onClick={handle_reset} data-testid="btn-new-signup">New Signup</button>
        </div>
        </> : (null)}
        {!submitted ? <>
        <div style={{flexDirection: 'column', gap: '4px', display: 'flex'}}>
        <span data-testid="label-name">Name *</span>
        <input placeholder="Enter your name" value={name} onChange={e => setName(e.target.value)} data-testid="input-name" />
        </div>
        </> : (null)}
        {!submitted ? <>
        <div style={{flexDirection: 'column', gap: '4px', display: 'flex'}}>
        <span data-testid="label-email">Email *</span>
        <input placeholder="Enter your email" value={email} onChange={e => setEmail(e.target.value)} data-testid="input-email" />
        </div>
        </> : (null)}
        {!submitted ? <>
        <div style={{flexDirection: 'column', gap: '4px', display: 'flex'}}>
        <span data-testid="label-age">Age</span>
        <input placeholder="Your age" value={age_val} onChange={e => setAge_val(e.target.value)} data-testid="input-age" />
        </div>
        </> : (null)}
        {!submitted ? <>
        <div style={{flexDirection: 'column', gap: '4px', display: 'flex'}}>
        <span data-testid="label-plan">Plan</span>
        <span data-testid="selected-plan">
        Selected: 
        {plan}
        </span>
        </div>
        </> : (null)}
        {!submitted ? <>
        <div style={{flexDirection: 'row', gap: '8px', alignItems: 'center', display: 'flex'}}>
        <button onClick={() => {
            setAccepted_terms(!accepted_terms);
        }} data-testid="btn-terms">{accepted_terms ? <span>YES</span> : (<span>NO</span>)}</button>
        <span data-testid="label-terms">I accept the terms</span>
        </div>
        {(errors !== "") ? <span data-testid="form-error">{errors}</span> : (null)}
        <div style={{flexDirection: 'row', gap: '8px', display: 'flex'}}>
        <button onClick={handle_submit} data-testid="btn-submit">Submit</button>
        <button onClick={handle_reset} data-testid="btn-reset">Reset</button>
        </div>
        <span data-testid="submit-count">
        Submissions: 
        {submit_count}
        </span>
        </> : (null)}
        </div>
    );
}