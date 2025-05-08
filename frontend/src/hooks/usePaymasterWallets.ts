import { useQuery } from "@tanstack/react-query";
import { useConfig } from "./useConfig";

export type Wallet = {
    address: string;
    balance: string;
};

export type PaymasterWallets = {
    deposit: Wallet;
    validating: Wallet;
};

/**
 * Fetches Paymaster Wallets from API
 */
const fetchPaymasterWallets = async (
    baseUrl: string,
): Promise<PaymasterWallets> => {
    const response = await fetch(`${baseUrl}/api/balances`);
    if (!response.ok) {
        throw new Error("Failed to fetch paymaster wallets");
    }
    const jsonResponse = await response.json();
    return jsonResponse.wallets;
};

/**
 * React Query hook to fetch Paymaster Wallets
 */
export const usePaymasterWallets = () => {
    const { apiBaseUrl, networkStatusRefetchIntervalS } = useConfig();
    return useQuery({
        queryKey: ["paymasterWallets"],
        queryFn: () => fetchPaymasterWallets(apiBaseUrl),
        refetchInterval: networkStatusRefetchIntervalS * 1000, // convert to ms
    });
};
