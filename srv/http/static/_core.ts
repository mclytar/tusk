/**
 * Denotes the result of a fallible operation.
 */
export class Result<T, E> {
    ok: boolean;
    value: T | E;

    is_ok(): boolean {
        return this.ok;
    }

    is_err(): boolean {
        return !this.ok;
    }

    unwrap(): T {
        if (this.ok) {
            return <T>this.value;
        } else {
            throw new Error("Tried to unwrap an error variant.");
        }
    }

    unwrap_err(): E {
        if (this.ok) {
            throw new Error("Tried to unwrap_err a non-error variant.");
        } else {
            return <E>this.value;
        }
    }
}
export function Ok<T, E>(value: T): Result<T, E> {
    let result = new Result<T, E>();
    result.ok = true;
    result.value = value;
    return result;
}

export function Err<T, E>(err: E): Result<T, E> {
    let result = new Result<T, E>();
    result.ok = false;
    result.value = err;
    return result;
}

/**
 * Denotes an optional value.
 */
export class Option<T> {
    some: boolean;
    value: T | null;

    is_ok(): boolean {
        return this.some;
    }

    is_err(): boolean {
        return !this.some;
    }

    unwrap(): T {
        if (this.some) {
            return <T>this.value;
        } else {
            throw new Error("Tried to unwrap a None variant.");
        }
    }
}
export function Some<T>(value: T): Option<T> {
    let result = new Option<T>();
    result.some = true;
    result.value = value;
    return result;
}

export function None<T>(): Option<T> {
    let result = new Option<T>();
    result.some = false;
    result.value = null;
    return result;
}