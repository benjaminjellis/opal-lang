-module(zier_list_helpers).
-export([foldl/3, foldr/3, nth/2]).

%% Adapt Zier's curried fun(X) -> fun(Acc) -> ... end
%% to the 2-arity fun(X, Acc) -> ... end that lists:foldl/foldr expect.

foldl(Fun, Acc0, List) ->
    lists:foldl(fun(X, Acc) -> (Fun(X))(Acc) end, Acc0, List).

foldr(Fun, Acc0, List) ->
    lists:foldr(fun(X, Acc) -> (Fun(X))(Acc) end, Acc0, List).

nth(N, List) when is_integer(N), N >= 0 ->
    case catch lists:nth(N + 1, List) of
        {'EXIT', _} -> none;
        Value -> {some, Value}
    end;
nth(_, _) ->
    none.
