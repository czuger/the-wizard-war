<?php
 /**
  *------
  * BGA framework: © Gregory Isabelli <gisabelli@boardgamearena.com> & Emmanuel Colin <ecolin@boardgamearena.com>
  * GuerreMagiciens implementation : © <Your name here> <Your email address here>
  * 
  * This code has been produced on the BGA studio platform for use on http://boardgamearena.com.
  * See http://en.boardgamearena.com/#!doc/Studio for more information.
  * -----
  * 
  * guerremagiciens.game.php
  *
  * This is the main file for your game logic.
  *
  * In this PHP file, you are going to defines the rules of the game.
  *
  */


require_once( APP_GAMEMODULE_PATH.'module/table/table.game.php' );


class GuerreMagiciens extends Table
{
	function __construct( )
	{
        // Your global variables labels:
        //  Here, you can assign labels to global variables you are using for this game.
        //  You can use any number of global variables with IDs between 10 and 99.
        //  If your game has options (variants), you also have to associate here a label to
        //  the corresponding ID in gameoptions.inc.php.
        // Note: afterwards, you can get/set the global variables with getGameStateValue/setGameStateInitialValue/setGameStateValue
        parent::__construct();
        
        self::initGameStateLabels( array( 
            //    "my_first_global_variable" => 10,
            //    "my_second_global_variable" => 11,
            //      ...
            //    "my_first_game_variant" => 100,
            //    "my_second_game_variant" => 101,
            //      ...
        ) );        
	}
	
    protected function getGameName( )
    {
		// Used for translations and stuff. Please do not modify.
        return "guerremagiciens";
    }	

    /*
        setupNewGame:
        
        This method is called only once, when a new game is launched.
        In this method, you must setup the game according to the game rules, so that
        the game is ready to be played.
    */
    protected function setupNewGame( $players, $options = array() )
    {    
        // Set the colors of the players with HTML color code
        // The default below is red/green/blue/orange/brown
        // The number of colors defined here must correspond to the maximum number of players allowed for the gams
        $gameinfos = self::getGameinfos();
        $default_colors = $gameinfos['player_colors'];
 
        // Create players
        // Note: if you added some extra field on "player" table in the database (dbmodel.sql), you can initialize it there.
        $sql = "INSERT INTO player (player_id, player_color, player_canal, player_name, player_avatar) VALUES ";
        $values = array();

        foreach( $players as $player_id => $player )
        {
            $color = array_shift( $default_colors );

            $subset_values = [];

            $subset_values[]= $player_id;
            $subset_values[]= "'".$color."'";
            $subset_values[]= "'".$player['player_canal']."'";
            $subset_values[]= "'".addslashes( $player['player_name'] )."'";
            $subset_values[]= "'".addslashes( $player['player_avatar'] )."'";

            $values[]= "(" . implode( $subset_values, ',' ) . ")";
        }
        $sql .= implode( $values, ',' );
        self::DbQuery( $sql );
        self::reattributeColorsBasedOnPreferences( $players, $gameinfos['player_colors'] );
        self::reloadPlayersBasicInfos();

        //
        // We dispatch the fanatics tokens
        //
        shuffle( $this -> fanatics_tokens_pool );
        // var_dump( $this -> fanatics );

        $sql = "INSERT INTO fanatics (player_id, fanatics_name, fanatics_strength, fanatics_code) VALUES ";
        $values = array();

        foreach( $players as $player_id => $player )
        {
            foreach(range(1, $this -> nb_fanatics[ count( $players ) ]) as $number) {

                $fanatic_token_code = array_shift( $this -> fanatics_tokens_pool );

                $fanatic_token = $this -> fanatics_tokens[ $fanatic_token_code ];

                $subset_values = [];

                $subset_values[]= $player_id;
                $subset_values[]= "'".$fanatic_token['side']."'";
                $subset_values[]= "'".$fanatic_token['strength']."'";
                $subset_values[]= "'".$fanatic_token_code."'";

                $values[]= "(" . implode( $subset_values, ',' ) . ")";
            }
        }
        $sql .= implode( $values, ',' );
        self::DbQuery( $sql );

        //
        // We create the magical items stock
        //
        $sql = "INSERT INTO magical_items_in_stock VALUES ( 'toratsa', 20, 0 )";
        self::DbQuery( $sql );

        $sql = "INSERT INTO magical_items_in_stock VALUES ( 'xephis', 25, 1 )";
        self::DbQuery( $sql );

        $sql = "INSERT INTO magical_items_in_stock VALUES ( 'yaboul', 35, 2 )";
        self::DbQuery( $sql );

        /************ Start the game initialization *****/

        // Init global values with their initial values
        //self::setGameStateInitialValue( 'my_first_global_variable', 0 );
        
        // Init game statistics
        // (note: statistics used in this file must be defined in your stats.inc.php file)
        //self::initStat( 'table', 'table_teststat1', 0 );    // Init a table statistics
        //self::initStat( 'player', 'player_teststat1', 0 );  // Init a player statistics (for all players)

        // TODO: setup the initial game situation here
       

        // Activate first player (which is in general a good idea :) )
        $this->activeNextPlayer();

        /************ End of the game initialization *****/
    }

    /*
        getAllDatas: 
        
        Gather all informations about current game situation (visible by the current player).
        
        The method is called each time the game interface is displayed to a player, ie:
        _ when the game starts
        _ when a player refreshes the game page (F5)
    */
    protected function getAllDatas()
    {
        $result = array();
    
        $current_player_id = self::getCurrentPlayerId();    // !! We must only return informations visible by this player !!
    
        // Get information about players
        // Note: you can retrieve some extra field you added for "player" table in "dbmodel.sql" if you need it.
        $sql = "SELECT player_id id, player_score score, player_money money, laboratories FROM player ";
  
        // TODO: Gather all information about current game situation (visible by player $current_player_id).
        $result['players'] = self::getCollectionFromDb( $sql );      

        return $result;
    }

    /*
        getGameProgression:
        
        Compute and return the current game progression.
        The number returned must be an integer beween 0 (=the game just started) and
        100 (= the game is finished or almost finished).
    
        This method is called each time we are in a game state with the "updateGameProgression" property set to true 
        (see states.inc.php)
    */
    function getGameProgression()
    {
        // TODO: compute and return the game progression

        return 0;
    }


//////////////////////////////////////////////////////////////////////////////
//////////// Utility functions
////////////    

    /*
        In this space, you can put any utility methods useful for your game logic
    */



//////////////////////////////////////////////////////////////////////////////
//////////// Player actions
//////////// 

    /*
        Each time a player is doing some game action, one of the methods below is called.
        (note: each method below must match an input method in guerremagiciens.action.php)
    */

    function gameFinishFanaticsDominanceSetup( $selected_fanatics )
    {
        // Check that this is the player's turn and that it is a "possible action" at this game state (see states.inc.php)
        self::checkAction( 'TownCriersExpense' ); 
        $player_id = self::getActivePlayerId();

        if( $selected_fanatics != '-1' ){
            $sql = "SELECT * FROM fanatics WHERE fanatics_code=".$selected_fanatics." AND player_id=".$player_id;
            $collection = self::getCollectionFromDb( $sql );
            $first_key = array_keys( $collection )[0];
            $result = $collection[$first_key];

            $sql = "UPDATE fanatics SET in_hall=1 WHERE internal_code=".$result['internal_code']." AND player_id=".$player_id;
            self::DbQuery( $sql );
        }

        // Add your game logic there
        $this->gamestate->nextState( 'TownCriersExpense' );
        
        // Notify all players about the card played
        self::notifyAllPlayers( "FanaticsDominanceSetupDone", clienttranslate( '${player_name} has selected his fanatics' ), array(
            'player_id' => $player_id,
            'player_name' => self::getActivePlayerName()
        ) );
    }

    function gameFinishTownCriersExpense( $total_expense )
    {
        // Check that this is the player's turn and that it is a "possible action" at this game state (see states.inc.php)
        self::checkAction( 'ItemsProduction' ); 
        
        $player_id = self::getActivePlayerId();

        $sql = "UPDATE player SET town_criers_expense=".$total_expense.", player_money=player_money-".$total_expense." WHERE player_id=".$player_id;
        self::DbQuery( $sql );

        $sql = "SELECT player_money FROM player WHERE player_id=".$player_id;
        $player_money = self::getUniqueValueFromDB( $sql );
        
        // Add your game logic there
        $this->gamestate->nextState( 'ItemsProduction' );
        
        // Notify all players about the card played
        self::notifyAllPlayers( "playerExpenseFinished", clienttranslate( '${player_name} has done his initial investisment' ), array(
            'player_id' => $player_id,
            'player_name' => self::getActivePlayerName(),
            'player_money' => $player_money
        ) );
    }

    function gameFinishProduceMagicalItem( $produced_items )
    {
        // Check that this is the player's turn and that it is a "possible action" at this game state (see states.inc.php)
        self::checkAction( 'FanaticsDominanceSetup' ); 
        
        $player_id = self::getActivePlayerId();

        self::dump( "Produced items = ", $produced_items );

        // $sql = "UPDATE player SET town_criers_expense=".$total_expense.", player_money=player_money-".$total_expense." WHERE player_id=".$player_id;
        // self::DbQuery( $sql );

        $magical_item_array = array('0' => 0, '1' => 0, '2' => 0);

        foreach ($produced_items as &$value) {
            $magical_item_array[$value] += 1;
        }

        $sql = "UPDATE player SET toratsa_in_stock=".$magical_item_array['0'].", xephis_in_stock=".$magical_item_array['1'].", yaboul_in_stock=".$magical_item_array['2']." WHERE player_id=".$player_id;
        self::DbQuery( $sql );
        
        // Add your game logic there

        // Clear fanatics selected by users.
        $sql = "DELETE FROM fanatics WHERE in_hall=1";
        self::DbQuery( $sql );

        $this->gamestate->nextState( 'FanaticsDominanceSetup' );
        
        // Notify all players about the card played
        self::notifyAllPlayers( "cardPlayed", clienttranslate( '${player_name} has finished it\'s production' ), array(
            'player_id' => $player_id,
            'player_name' => self::getActivePlayerName()
        ) );
    }   

    /*
    
    Example:

    function playCard( $card_id )
    {
        // Check that this is the player's turn and that it is a "possible action" at this game state (see states.inc.php)
        self::checkAction( 'playCard' ); 
        
        $player_id = self::getActivePlayerId();
        
        // Add your game logic to play a card there 
        ...
        
        // Notify all players about the card played
        self::notifyAllPlayers( "cardPlayed", clienttranslate( '${player_name} plays ${card_name}' ), array(
            'player_id' => $player_id,
            'player_name' => self::getActivePlayerName(),
            'card_name' => $card_name,
            'card_id' => $card_id
        ) );
          
    }
    
    */

    
//////////////////////////////////////////////////////////////////////////////
//////////// Game state arguments
////////////

    /*
        Here, you can create methods defined as "game state arguments" (see "args" property in states.inc.php).
        These methods function is to return some additional information that is specific to the current
        game state.
    */

    function argFanaticsDominanceSetup()
    {
        // Get some values from the current game situation in database...
    
        // return values:
        $sql = "SELECT * FROM `fanatics` ORDER BY fanatics_code";
        $user_fanatics = self::getObjectListFromDB( $sql );

        return [ 'user_fanatics' => $user_fanatics, 'fanatics_tokens' => $this -> fanatics_tokens ];
    }    

    function argTownCriersExpense()
    {
        // Get some values from the current game situation in database...
    
        // return values:
        return [ 'global_expense_input_tokens' => $this -> global_expense_input_tokens ];
    }    

    function argItemsProduction()
    {
        // Get some values from the current game situation in database...
    
        // return values:
        $sql = "SELECT * FROM `magical_items_in_stock` ORDER BY magical_item_code";
        return self::getObjectListFromDB( $sql );
    }    

    /*
    
    Example for game state "MyGameState":
    
    function argMyGameState()
    {
        // Get some values from the current game situation in database...
    
        // return values:
        return array(
            'variable1' => $value1,
            'variable2' => $value2,
            ...
        );
    }    
    */



//////////////////////////////////////////////////////////////////////////////
//////////// Game state actions
////////////

    /*
        Here, you can create methods defined as "game state actions" (see "action" property in states.inc.php).
        The action method of state X is called everytime the current game state is set to X.
    */

    function stFanaticsDominanceSetup()
    {
        // Do some stuff ...
        $this->gamestate->setAllPlayersMultiactive();

        // (very often) go to another gamestate
        // $this->gamestate->nextState( 'some_gamestate_transition' );
    }    
    
    function stTownCriersExpense()
    {
        // Do some stuff ...
        $this->gamestate->setAllPlayersMultiactive();

        // (very often) go to another gamestate
        // $this->gamestate->nextState( 'some_gamestate_transition' );
    }    

    /*
    
    Example for game state "MyGameState":

    function stMyGameState()
    {
        // Do some stuff ...
        
        // (very often) go to another gamestate
        $this->gamestate->nextState( 'some_gamestate_transition' );
    }    
    */

//////////////////////////////////////////////////////////////////////////////
//////////// Zombie
////////////

    /*
        zombieTurn:
        
        This method is called each time it is the turn of a player who has quit the game (= "zombie" player).
        You can do whatever you want in order to make sure the turn of this player ends appropriately
        (ex: pass).
    */

    function zombieTurn( $state, $active_player )
    {
    	$statename = $state['name'];
    	
        if ($state['type'] === "activeplayer") {
            switch ($statename) {
                default:
                    $this->gamestate->nextState( "zombiePass" );
                	break;
            }

            return;
        }

        if ($state['type'] === "multipleactiveplayer") {
            // Make sure player is in a non blocking status for role turn
            $this->gamestate->setPlayerNonMultiactive( $active_player, '' );
            
            return;
        }

        throw new feException( "Zombie mode not supported at this game state: ".$statename );
    }
    
///////////////////////////////////////////////////////////////////////////////////:
////////// DB upgrade
//////////

    /*
        upgradeTableDb:
        
        You don't have to care about this until your game has been published on BGA.
        Once your game is on BGA, this method is called everytime the system detects a game running with your old
        Database scheme.
        In this case, if you change your Database scheme, you just have to apply the needed changes in order to
        update the game database and allow the game to continue to run with your new version.
    
    */
    
    function upgradeTableDb( $from_version )
    {
        // $from_version is the current version of this game database, in numerical form.
        // For example, if the game was running with a release of your game named "140430-1345",
        // $from_version is equal to 1404301345
        
        // Example:
//        if( $from_version <= 1404301345 )
//        {
//            $sql = "ALTER TABLE xxxxxxx ....";
//            self::DbQuery( $sql );
//        }
//        if( $from_version <= 1405061421 )
//        {
//            $sql = "CREATE TABLE xxxxxxx ....";
//            self::DbQuery( $sql );
//        }
//        // Please add your future database scheme changes here
//
//


    }    
}
