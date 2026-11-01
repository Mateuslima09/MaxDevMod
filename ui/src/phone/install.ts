import { installPhoneSocket } from './socket'

/** Whether the phone page has its way to the PC (the key in its address, or kept). Imported first of all (main.tsx):
 * the cab's code it runs looks for the host as it loads. */
export const PHONE_CONNECTS = installPhoneSocket()
