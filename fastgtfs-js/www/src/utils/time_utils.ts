export const UPDATE_PERIOD = 2000;

export function getSecondsSinceMidnight(): number {
    const now = new Date();

    const then = new Date(now.getFullYear(), now.getMonth(), now.getDate(), 0, 0, 0);

    return (now.getTime() - then.getTime()) / 1000;
}

export function getDateYYYYMMDD(): string {
    const todayDate = new Date().toISOString().slice(0, 10);
    return todayDate.replaceAll('-', '');
}
