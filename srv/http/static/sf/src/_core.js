/**
 * Denotes the result of a fallible operation.
 */
export class Result {
    ok;
    value;
    is_ok() {
        return this.ok;
    }
    is_err() {
        return !this.ok;
    }
    unwrap() {
        if (this.ok) {
            return this.value;
        }
        else {
            throw new Error("Tried to unwrap an error variant.");
        }
    }
    unwrap_err() {
        if (this.ok) {
            throw new Error("Tried to unwrap_err a non-error variant.");
        }
        else {
            return this.value;
        }
    }
}
export function Ok(value) {
    let result = new Result();
    result.ok = true;
    result.value = value;
    return result;
}
export function Err(err) {
    let result = new Result();
    result.ok = false;
    result.value = err;
    return result;
}
/**
 * Denotes an optional value.
 */
export class Option {
    some;
    value;
    is_ok() {
        return this.some;
    }
    is_err() {
        return !this.some;
    }
    unwrap() {
        if (this.some) {
            return this.value;
        }
        else {
            throw new Error("Tried to unwrap a None variant.");
        }
    }
}
export function Some(value) {
    let result = new Option();
    result.some = true;
    result.value = value;
    return result;
}
export function None() {
    let result = new Option();
    result.some = false;
    result.value = null;
    return result;
}
//# sourceMappingURL=_core.js.map