import { useQuery } from "@tanstack/react-query";

export type Wallet = {
    address: string;
    balance: string;
};

export type PaymasterWallets = {
    deposit: Wallet;
    validating: Wallet;
};

const API_BASE_URL = import.meta.env.API_BASE_URL || "http://localhost:3000";

/**
 * Fetches Paymaster Wallets from API
 */
const fetchPaymasterWallets = async (): Promise<PaymasterWallets> => {
    const response = await fetch(`${API_BASE_URL}/api/balances`);
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
    return useQuery({
        queryKey: ["paymasterWallets"],
        queryFn: fetchPaymasterWallets,
        refetchInterval: 10000, // Auto-refresh every 30s
    });
};