import { useQuery } from "@tanstack/react-query";

export type OperatorStatus = {
    operator_id: string,
    operator_address: string,
    status: string,
}

export type DepositInfo = {
    deposit_request_txid: string,
    deposit_txid: string,
    mint_txid: string,
    status: string,
}

export type WithdrawalInfo = {
    withdrawal_request_txid: string,
    fulfillment_txid: string,
    status: string,
}

export type ReimbursementInfo = {
    claim_txid: string,
    challenge_step: string,
    payout_txid: string,
    status: string,
}

export type BridgeStatus = {
    operators: OperatorStatus[],
    deposits: DepositInfo[],
    withdrawals: WithdrawalInfo[],
    reimbursements: ReimbursementInfo[],
};

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || "http://localhost:3000";
const REFETCH_INTERVAL_S = import.meta.env.VITE_BRIDGE_STATUS_REFETCH_INTERVAL_S || "http://localhost:3000";
const fetchStatus = async (): Promise<BridgeStatus> => {
    const response = await fetch(`${API_BASE_URL}/api/bridge_status`);
    if (!response.ok) {
        throw new Error("Failed to fetch status");
    }
    return response.json();
};

export const useBridgeStatus = () => {
    return useQuery({
        queryKey: ["bridgeStatus"],
        queryFn: fetchStatus,
        refetchInterval: REFETCH_INTERVAL_S * 1000,
    });
};